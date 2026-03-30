use anyhow::{bail, Context, Result};
use chrono::Utc;
use serde::Serialize;
use sqlx::{Row, SqlitePool};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use tokio::fs::OpenOptions;
use tokio::io::AsyncWriteExt;
use tokio::process::Command;
use uuid::Uuid;

use crate::{
    agents::{self, AgentProfile},
    config::Paths,
    db,
    memory::MemoryStore,
    models::{
        AgentExecution, HiddenScenarioSummary, IntentPackage, RunEvent, RunRecord,
        ScenarioResult, Spec, TwinEnvironment, TwinService, ValidationSummary,
    },
    policy,
    scenarios,
    setup::command_available,
    spec, workspace,
};

#[derive(Debug, Clone)]
pub struct AppContext {
    pub paths: Paths,
    pub pool: SqlitePool,
    pub memory_store: MemoryStore,
}

#[derive(Debug, Clone)]
pub struct RunRequest {
    pub spec_path: String,
    pub product: String,
}

#[derive(Debug, Clone)]
struct ExecutionOutput {
    validation: ValidationSummary,
    hidden_scenarios: HiddenScenarioSummary,
}

#[derive(Debug, Clone)]
struct CommandOutcome {
    success: bool,
    code: Option<i32>,
    stdout: String,
    stderr: String,
}

impl AppContext {
    pub async fn bootstrap() -> Result<Self> {
        let paths = Paths::discover()?;
        tokio::fs::create_dir_all(&paths.factory).await?;
        tokio::fs::create_dir_all(&paths.logs).await?;
        tokio::fs::create_dir_all(&paths.artifacts).await?;
        tokio::fs::create_dir_all(&paths.workspaces).await?;
        tokio::fs::create_dir_all(&paths.memory).await?;
        tokio::fs::create_dir_all(&paths.specs).await?;
        tokio::fs::create_dir_all(&paths.scenarios).await?;
        tokio::fs::create_dir_all(&paths.products).await?;

        let pool = db::init_db(&paths).await?;
        let memory_store = MemoryStore::new(paths.memory.clone());
        Ok(Self {
            paths,
            pool,
            memory_store,
        })
    }

    pub async fn start_run(&self, req: RunRequest) -> Result<RunRecord> {
        let spec_obj = spec::load_spec(&req.spec_path)?;
        let product_source = self.paths.products.join(&req.product);
        if !product_source.exists() {
            bail!("product not found: {}", product_source.display());
        }

        let run_id = Uuid::new_v4().to_string();
        let now = Utc::now();
        let log_path = self.run_log_path(&run_id);

        self.insert_run(&run_id, &spec_obj.id, &req.product, "queued", now)
            .await?;
        self.record_event(
            &run_id,
            "queued",
            "orchestrator",
            "queued",
            &format!(
                "Created run for spec {} against product {}",
                spec_obj.id, req.product
            ),
            &log_path,
        )
        .await?;

        match self.execute_run(&run_id, &req, &spec_obj, &log_path).await {
            Ok(output) => {
                let final_status = if output.validation.passed && output.hidden_scenarios.passed {
                    "completed"
                } else {
                    "completed_with_issues"
                };
                self.update_run_status(&run_id, final_status).await?;
                self.record_event(
                    &run_id,
                    "complete",
                    "orchestrator",
                    if final_status == "completed" {
                        "complete"
                    } else {
                        "warning"
                    },
                    &format!("Run finished with status {}", final_status),
                    &log_path,
                )
                .await?;
                self.package_artifacts(&run_id).await?;
            }
            Err(error) => {
                let message = error.to_string();
                self.update_run_status(&run_id, "failed").await?;
                self.record_event(
                    &run_id,
                    "complete",
                    "orchestrator",
                    "failed",
                    &message,
                    &log_path,
                )
                .await?;
                let _ = self.package_artifacts(&run_id).await;
            }
        }

        self.get_run(&run_id)
            .await?
            .with_context(|| format!("run not found after execution: {run_id}"))
    }

    async fn execute_run(
        &self,
        run_id: &str,
        req: &RunRequest,
        spec_obj: &Spec,
        log_path: &Path,
    ) -> Result<ExecutionOutput> {
        let profiles = agents::load_profiles(&self.paths.factory.join("agents").join("profiles"))?;
        let workspace_root = workspace::create_run_workspace(&self.paths.workspaces, run_id).await?;
        let run_dir = workspace_root.join("run");
        let agents_dir = run_dir.join("agents");
        tokio::fs::create_dir_all(&agents_dir).await?;
        tokio::fs::copy(&req.spec_path, run_dir.join("spec.yaml"))
            .await
            .with_context(|| format!("copying spec snapshot for run {run_id}"))?;

        let mut agent_executions = Vec::new();

        self.update_run_status(run_id, "intake").await?;
        self.record_event(
            run_id,
            "intake",
            "scout",
            "running",
            &format!("Loading spec '{}'", spec_obj.title),
            log_path,
        )
        .await?;
        let memory_hits = self.memory_store.retrieve_context(&spec_obj.title).await?;
        let intent = self.scout_intake(spec_obj, &memory_hits).await?;
        self.write_json_file(&run_dir.join("intent.json"), &intent).await?;
        self.write_agent_execution(
            &profiles,
            "scout",
            &format!("Interpret spec {} and prepare a normalized intent package.", spec_obj.id),
            "Parsed the spec and produced an implementation intent package.",
            &serde_json::to_string_pretty(&intent)?,
            &run_dir,
            &mut agent_executions,
        )
        .await?;
        self.record_event(
            run_id,
            "intake",
            "scout",
            "complete",
            &format!(
                "Intent package ready with {} recommended steps",
                intent.recommended_steps.len()
            ),
            log_path,
        )
        .await?;

        self.update_run_status(run_id, "memory").await?;
        self.record_event(
            run_id,
            "memory",
            "coobie",
            "running",
            "Retrieving prior factory context",
            log_path,
        )
        .await?;
        tokio::fs::write(
            run_dir.join("memory_context.md"),
            format_memory_context(&memory_hits),
        )
        .await?;
        self.write_agent_execution(
            &profiles,
            "coobie",
            &format!("Retrieve prior context for spec '{}'", spec_obj.title),
            &format!("Collected {} memory hit(s).", memory_hits.len()),
            &format_memory_context(&memory_hits),
            &run_dir,
            &mut agent_executions,
        )
        .await?;
        self.record_event(
            run_id,
            "memory",
            "coobie",
            "complete",
            &format!("Captured {} memory hit(s)", memory_hits.len()),
            log_path,
        )
        .await?;

        self.update_run_status(run_id, "workspace").await?;
        self.record_event(
            run_id,
            "workspace",
            "keeper",
            "running",
            "Verifying workspace boundaries",
            log_path,
        )
        .await?;
        let staged_product = workspace::stage_product_workspace(
            &self.paths.products,
            &workspace_root,
            &req.product,
        )
        .await?;
        policy::ensure_path_within(&workspace_root, &staged_product)?;
        self.write_agent_execution(
            &profiles,
            "keeper",
            "Verify that all product writes stay inside the staged run workspace.",
            "Workspace boundaries verified.",
            &format!(
                "workspace_root={}\nstaged_product={}\npolicy=within_workspace",
                workspace_root.display(),
                staged_product.display()
            ),
            &run_dir,
            &mut agent_executions,
        )
        .await?;
        self.record_event(
            run_id,
            "workspace",
            "keeper",
            "complete",
            "Workspace boundaries verified",
            log_path,
        )
        .await?;

        self.update_run_status(run_id, "implementation").await?;
        self.record_event(
            run_id,
            "implementation",
            "mason",
            "running",
            "Drafting implementation plan for the staged product",
            log_path,
        )
        .await?;
        let implementation_plan = build_implementation_plan(spec_obj, &intent, &memory_hits, &staged_product);
        tokio::fs::write(run_dir.join("implementation_plan.md"), &implementation_plan).await?;
        self.write_agent_execution(
            &profiles,
            "mason",
            &format!(
                "Prepare an implementation plan for product '{}' using the staged workspace.",
                req.product
            ),
            "Prepared a local implementation plan for the staged product copy.",
            &implementation_plan,
            &run_dir,
            &mut agent_executions,
        )
        .await?;
        self.record_event(
            run_id,
            "implementation",
            "mason",
            "complete",
            &format!("Staged product copy at {}", staged_product.display()),
            log_path,
        )
        .await?;

        self.update_run_status(run_id, "tools").await?;
        self.record_event(
            run_id,
            "tools",
            "piper",
            "running",
            "Reviewing tool and MCP availability",
            log_path,
        )
        .await?;
        let tool_plan = self.build_tool_plan();
        tokio::fs::write(run_dir.join("tool_plan.md"), &tool_plan).await?;
        self.write_agent_execution(
            &profiles,
            "piper",
            "Summarize the configured provider and MCP tool surface for this run.",
            "Captured current tool and MCP availability for the run.",
            &tool_plan,
            &run_dir,
            &mut agent_executions,
        )
        .await?;
        self.record_event(
            run_id,
            "tools",
            "piper",
            "complete",
            "Tool and MCP plan captured",
            log_path,
        )
        .await?;

        self.update_run_status(run_id, "twin").await?;
        self.record_event(
            run_id,
            "twin",
            "ash",
            "running",
            "Provisioning local twin environment",
            log_path,
        )
        .await?;
        let twin = self.build_twin_environment(run_id, spec_obj);
        self.write_json_file(&run_dir.join("twin.json"), &twin).await?;
        self.write_agent_execution(
            &profiles,
            "ash",
            "Provision a safe local twin environment for validation and hidden scenario work.",
            &format!("Provisioned {} twin service(s).", twin.services.len()),
            &serde_json::to_string_pretty(&twin)?,
            &run_dir,
            &mut agent_executions,
        )
        .await?;
        self.record_event(
            run_id,
            "twin",
            "ash",
            "complete",
            &format!("Provisioned {} twin service(s)", twin.services.len()),
            log_path,
        )
        .await?;

        self.update_run_status(run_id, "validation").await?;
        self.record_event(
            run_id,
            "validation",
            "bramble",
            "running",
            "Running visible validation",
            log_path,
        )
        .await?;
        let validation = self.run_visible_validation(&workspace_root, &staged_product).await?;
        self.write_json_file(&run_dir.join("validation.json"), &validation)
            .await?;
        self.write_agent_execution(
            &profiles,
            "bramble",
            "Run the visible validation loop over the staged product copy.",
            &format!(
                "Visible validation finished with {} check(s).",
                validation.results.len()
            ),
            &serde_json::to_string_pretty(&validation)?,
            &run_dir,
            &mut agent_executions,
        )
        .await?;
        self.record_event(
            run_id,
            "validation",
            "bramble",
            if validation.passed {
                "complete"
            } else {
                "warning"
            },
            &format!(
                "Visible validation finished: {} checks, {} passed",
                validation.results.len(),
                validation.results.iter().filter(|result| result.passed).count()
            ),
            log_path,
        )
        .await?;

        self.update_run_status(run_id, "hidden_scenarios").await?;
        self.record_event(
            run_id,
            "hidden_scenarios",
            "sable",
            "running",
            "Evaluating hidden scenarios",
            log_path,
        )
        .await?;
        let predicted_final_status = if validation.passed {
            "completed"
        } else {
            "completed_with_issues"
        };
        let events_so_far = self.list_run_events(run_id).await?;
        let hidden_definitions = scenarios::load_hidden_scenarios(&self.paths.scenarios, &spec_obj.id)?;
        let hidden_scenarios = scenarios::evaluate_hidden_scenarios(
            &hidden_definitions,
            predicted_final_status,
            &events_so_far,
            &validation,
            &twin,
            &agent_executions,
            &run_dir,
        );
        self.write_json_file(&run_dir.join("hidden_scenarios.json"), &hidden_scenarios)
            .await?;
        self.write_agent_execution(
            &profiles,
            "sable",
            "Execute hidden scenarios from the protected scenario store and compare the run against them.",
            &format!("Hidden scenarios passed: {}", hidden_scenarios.passed),
            &serde_json::to_string_pretty(&hidden_scenarios)?,
            &run_dir,
            &mut agent_executions,
        )
        .await?;
        self.record_event(
            run_id,
            "hidden_scenarios",
            "sable",
            if hidden_scenarios.passed {
                "complete"
            } else {
                "warning"
            },
            &format!(
                "Hidden scenario evaluation finished: {} scenario(s)",
                hidden_scenarios.results.len()
            ),
            log_path,
        )
        .await?;

        self.memory_store
            .store(
                &format!("run-{}", run_id),
                vec![
                    "run".to_string(),
                    spec_obj.id.clone(),
                    req.product.clone(),
                    if validation.passed && hidden_scenarios.passed {
                        "completed".to_string()
                    } else {
                        "completed-with-issues".to_string()
                    },
                ],
                &format!("Run {} for {}", run_id, spec_obj.title),
                &format!(
                    "Spec: {}\nProduct: {}\nVisible validation passed: {}\nHidden scenarios passed: {}\nRecommended steps: {}\n\nTop memory hits:\n{}",
                    spec_obj.id,
                    req.product,
                    validation.passed,
                    hidden_scenarios.passed,
                    intent.recommended_steps.join(", "),
                    memory_hits.join("\n\n")
                ),
            )
            .await?;
        self.record_event(
            run_id,
            "memory",
            "coobie",
            "complete",
            "Stored run summary back into local memory",
            log_path,
        )
        .await?;

        self.update_run_status(run_id, "artifacts").await?;
        self.record_event(
            run_id,
            "artifacts",
            "flint",
            "running",
            "Packaging run artifacts",
            log_path,
        )
        .await?;
        self.write_agent_execution(
            &profiles,
            "flint",
            "Collect outputs, logs, and evaluation evidence into a portable artifact bundle.",
            "Prepared bundle contents for packaging.",
            &list_run_directory(&run_dir)?.join("\n"),
            &run_dir,
            &mut agent_executions,
        )
        .await?;
        self.package_artifacts(run_id).await?;
        self.record_event(
            run_id,
            "artifacts",
            "flint",
            "complete",
            "Artifact bundle refreshed",
            log_path,
        )
        .await?;

        Ok(ExecutionOutput {
            validation,
            hidden_scenarios,
        })
    }

    async fn scout_intake(&self, spec_obj: &Spec, memory_hits: &[String]) -> Result<IntentPackage> {
        let mut ambiguity_notes = Vec::new();
        if spec_obj.outputs.is_empty() {
            ambiguity_notes.push("Spec does not describe concrete outputs yet".to_string());
        }
        if spec_obj.acceptance_criteria.is_empty() {
            ambiguity_notes.push("Spec is missing acceptance criteria".to_string());
        }
        if memory_hits
            .iter()
            .any(|hit| hit.contains("No memories found") || hit.contains("Memory not initialized"))
        {
            ambiguity_notes.push("No strong prior memory matched this spec".to_string());
        }

        Ok(IntentPackage {
            spec_id: spec_obj.id.clone(),
            summary: format!("Implement {}", spec_obj.title),
            ambiguity_notes,
            recommended_steps: vec![
                "Retrieve prior patterns with Coobie".into(),
                "Stage an isolated product workspace".into(),
                "Run visible validation before scenario work".into(),
                "Evaluate hidden scenarios with Sable".into(),
                "Package evidence for human review".into(),
            ],
        })
    }

    async fn run_visible_validation(
        &self,
        workspace_root: &Path,
        staged_product: &Path,
    ) -> Result<ValidationSummary> {
        let mut results = Vec::new();
        let run_dir = workspace_root.join("run");
        let workspace_ok = staged_product.exists() && run_dir.exists();
        results.push(ScenarioResult {
            scenario_id: "workspace_layout".to_string(),
            passed: workspace_ok,
            details: format!(
                "workspace={} run_dir={}",
                staged_product.display(),
                run_dir.display()
            ),
        });

        let validation_log_path = run_dir.join("validation_output.log");
        let mut output_chunks = Vec::new();

        let cargo_manifest = staged_product.join("Cargo.toml");
        if cargo_manifest.exists() {
            let outcome = self
                .run_command_capture("cargo", &["check", "--quiet"], staged_product)
                .await?;
            output_chunks.push(format_command_output("cargo check --quiet", &outcome));
            results.push(ScenarioResult {
                scenario_id: "cargo_check".to_string(),
                passed: outcome.success,
                details: command_detail("cargo check --quiet", &outcome),
            });
        } else {
            let package_json = staged_product.join("package.json");
            if package_json.exists() {
                let scripts = detect_package_scripts(&package_json)?;
                if scripts.contains(&"build".to_string()) {
                    let outcome = self
                        .run_command_capture("npm", &["run", "build"], staged_product)
                        .await?;
                    output_chunks.push(format_command_output("npm run build", &outcome));
                    results.push(ScenarioResult {
                        scenario_id: "npm_build".to_string(),
                        passed: outcome.success,
                        details: command_detail("npm run build", &outcome),
                    });
                } else if scripts.contains(&"test".to_string()) {
                    let outcome = self
                        .run_command_capture("npm", &["run", "test"], staged_product)
                        .await?;
                    output_chunks.push(format_command_output("npm run test", &outcome));
                    results.push(ScenarioResult {
                        scenario_id: "npm_test".to_string(),
                        passed: outcome.success,
                        details: command_detail("npm run test", &outcome),
                    });
                } else {
                    results.push(ScenarioResult {
                        scenario_id: "build_manifest".to_string(),
                        passed: true,
                        details: "package.json found but no build/test script is defined".to_string(),
                    });
                }
            } else {
                results.push(ScenarioResult {
                    scenario_id: "build_manifest".to_string(),
                    passed: true,
                    details: "No Cargo.toml or package.json found; visible validation skipped".to_string(),
                });
            }
        }

        if !output_chunks.is_empty() {
            tokio::fs::write(&validation_log_path, output_chunks.join("\n\n"))
                .await
                .with_context(|| format!("writing validation log {}", validation_log_path.display()))?;
        }

        Ok(ValidationSummary {
            passed: results.iter().all(|result| result.passed),
            results,
        })
    }

    async fn run_command_capture(
        &self,
        program: &str,
        args: &[&str],
        cwd: &Path,
    ) -> Result<CommandOutcome> {
        let output = Command::new(program)
            .args(args)
            .current_dir(cwd)
            .output()
            .await
            .with_context(|| format!("running {} in {}", program, cwd.display()))?;

        Ok(CommandOutcome {
            success: output.status.success(),
            code: output.status.code(),
            stdout: String::from_utf8_lossy(&output.stdout).trim().to_string(),
            stderr: String::from_utf8_lossy(&output.stderr).trim().to_string(),
        })
    }

    async fn write_agent_execution(
        &self,
        profiles: &HashMap<String, AgentProfile>,
        agent_name: &str,
        prompt: &str,
        summary: &str,
        output: &str,
        run_dir: &Path,
        agent_executions: &mut Vec<AgentExecution>,
    ) -> Result<()> {
        let profile = profiles
            .get(agent_name)
            .with_context(|| format!("agent profile not found: {agent_name}"))?;
        let execution = agents::build_execution(profile, &self.paths.setup, prompt, summary, output);
        let agents_dir = run_dir.join("agents");
        tokio::fs::create_dir_all(&agents_dir).await?;
        self.write_json_file(&agents_dir.join(format!("{agent_name}.json")), &execution)
            .await?;

        agent_executions.retain(|existing| existing.agent_name != agent_name);
        agent_executions.push(execution);
        agent_executions.sort_by(|left, right| left.agent_name.cmp(&right.agent_name));
        self.write_json_file(&agents_dir.join("index.json"), agent_executions)
            .await?;
        self.write_json_file(&run_dir.join("agent_executions.json"), agent_executions)
            .await?;
        Ok(())
    }

    fn build_tool_plan(&self) -> String {
        let mut lines = vec![
            "Tool Plan".to_string(),
            "=========".to_string(),
            format!("Setup: {}", self.paths.setup.setup.name),
            format!("Default provider: {}", self.paths.setup.providers.default),
        ];

        if let Some(mcp) = &self.paths.setup.mcp {
            lines.push(String::new());
            lines.push("MCP Servers".to_string());
            lines.push("-----------".to_string());
            for server in &mcp.servers {
                lines.push(format!(
                    "- {} via {} {} (available={})",
                    server.name,
                    server.command,
                    server.args.join(" "),
                    command_available(&server.command)
                ));
            }
        }

        lines.push(String::new());
        lines.push("Host Commands".to_string());
        lines.push("-------------".to_string());
        for command in ["cargo", "node", "npm", "docker", "podman", "openclaw"] {
            lines.push(format!("- {} available={}", command, command_available(command)));
        }

        lines.join("\n") + "\n"
    }

    fn build_twin_environment(&self, run_id: &str, spec_obj: &Spec) -> TwinEnvironment {
        let mut services = vec![
            TwinService {
                name: "workspace_fs".to_string(),
                kind: "filesystem".to_string(),
                status: "ready".to_string(),
                details: self.paths.workspaces.display().to_string(),
            },
            TwinService {
                name: "state_db".to_string(),
                kind: "sqlite".to_string(),
                status: "ready".to_string(),
                details: self.paths.db_file.display().to_string(),
            },
            TwinService {
                name: "memory_store".to_string(),
                kind: "memory".to_string(),
                status: "ready".to_string(),
                details: self.paths.memory.display().to_string(),
            },
        ];

        if self.paths.setup.setup.anythingllm.unwrap_or(false) {
            services.push(TwinService {
                name: "anythingllm".to_string(),
                kind: "rag".to_string(),
                status: "configured".to_string(),
                details: "AnythingLLM is enabled for this setup.".to_string(),
            });
        }
        if self.paths.setup.setup.openclaw.unwrap_or(false) {
            services.push(TwinService {
                name: "openclaw".to_string(),
                kind: "orchestrator".to_string(),
                status: "configured".to_string(),
                details: "OpenClaw is enabled for this setup.".to_string(),
            });
        }

        let fingerprint = self
            .paths
            .setup
            .machine
            .as_ref()
            .and_then(|machine| machine.fingerprint.as_ref());
        if fingerprint.map(|fingerprint| fingerprint.docker || fingerprint.podman).unwrap_or(false) {
            services.push(TwinService {
                name: "container_runtime".to_string(),
                kind: "container".to_string(),
                status: "ready".to_string(),
                details: "Container runtime available for twin workloads.".to_string(),
            });
        }

        for dependency in &spec_obj.dependencies {
            let name = dependency
                .to_lowercase()
                .chars()
                .map(|ch| if ch.is_ascii_alphanumeric() { ch } else { '_' })
                .collect::<String>();
            if services.iter().any(|service| service.name == name) {
                continue;
            }
            services.push(TwinService {
                name,
                kind: "dependency".to_string(),
                status: "simulated".to_string(),
                details: format!("Synthetic twin stub for dependency {}", dependency),
            });
        }

        TwinEnvironment {
            name: format!("run-{run_id}-twin"),
            status: "ready".to_string(),
            services,
            created_at: Utc::now(),
        }
    }

    async fn insert_run(
        &self,
        run_id: &str,
        spec_id: &str,
        product: &str,
        status: &str,
        now: chrono::DateTime<Utc>,
    ) -> Result<()> {
        sqlx::query(
            r#"
            INSERT INTO runs (run_id, spec_id, product, status, created_at, updated_at)
            VALUES (?1, ?2, ?3, ?4, ?5, ?6)
            "#,
        )
        .bind(run_id)
        .bind(spec_id)
        .bind(product)
        .bind(status)
        .bind(now.to_rfc3339())
        .bind(now.to_rfc3339())
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    async fn update_run_status(&self, run_id: &str, status: &str) -> Result<()> {
        sqlx::query(
            r#"
            UPDATE runs
            SET status = ?2, updated_at = ?3
            WHERE run_id = ?1
            "#,
        )
        .bind(run_id)
        .bind(status)
        .bind(Utc::now().to_rfc3339())
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    async fn record_event(
        &self,
        run_id: &str,
        phase: &str,
        agent: &str,
        status: &str,
        message: &str,
        log_path: &Path,
    ) -> Result<()> {
        let created_at = Utc::now();
        sqlx::query(
            r#"
            INSERT INTO run_events (run_id, phase, agent, status, message, created_at)
            VALUES (?1, ?2, ?3, ?4, ?5, ?6)
            "#,
        )
        .bind(run_id)
        .bind(phase)
        .bind(agent)
        .bind(status)
        .bind(message)
        .bind(created_at.to_rfc3339())
        .execute(&self.pool)
        .await?;

        let mut file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(log_path)
            .await
            .with_context(|| format!("opening run log {}", log_path.display()))?;
        let line = format!(
            "{} [{}] {} {}: {}\n",
            created_at.to_rfc3339(),
            phase,
            agent,
            status,
            message
        );
        file.write_all(line.as_bytes()).await?;
        Ok(())
    }

    async fn write_json_file<T: Serialize>(&self, path: &Path, value: &T) -> Result<()> {
        let content = serde_json::to_string_pretty(value)?;
        tokio::fs::write(path, content)
            .await
            .with_context(|| format!("writing json file {}", path.display()))?;
        Ok(())
    }

    fn run_log_path(&self, run_id: &str) -> PathBuf {
        self.paths.logs.join(format!("{run_id}.log"))
    }

    fn workspace_root(&self, run_id: &str) -> PathBuf {
        self.paths.workspaces.join(run_id)
    }

    fn run_dir(&self, run_id: &str) -> PathBuf {
        self.workspace_root(run_id).join("run")
    }

    pub async fn get_run(&self, run_id: &str) -> Result<Option<RunRecord>> {
        let row = sqlx::query(
            "SELECT run_id, spec_id, product, status, created_at, updated_at FROM runs WHERE run_id = ?"
        )
        .bind(run_id)
        .fetch_optional(&self.pool)
        .await?;

        let Some(row) = row else {
            return Ok(None);
        };

        let run = RunRecord {
            run_id: row.get::<String, _>("run_id"),
            spec_id: row.get::<String, _>("spec_id"),
            product: row.get::<String, _>("product"),
            status: row.get::<String, _>("status"),
            created_at: chrono::DateTime::parse_from_rfc3339(
                row.get::<String, _>("created_at").as_str(),
            )?
            .with_timezone(&Utc),
            updated_at: chrono::DateTime::parse_from_rfc3339(
                row.get::<String, _>("updated_at").as_str(),
            )?
            .with_timezone(&Utc),
        };

        Ok(Some(run))
    }

    pub async fn list_runs(&self, limit: i64) -> Result<Vec<RunRecord>> {
        let rows = sqlx::query(
            "SELECT run_id, spec_id, product, status, created_at, updated_at FROM runs ORDER BY created_at DESC LIMIT ?"
        )
        .bind(limit)
        .fetch_all(&self.pool)
        .await?;

        let mut runs = Vec::new();
        for row in rows {
            runs.push(RunRecord {
                run_id: row.get::<String, _>("run_id"),
                spec_id: row.get::<String, _>("spec_id"),
                product: row.get::<String, _>("product"),
                status: row.get::<String, _>("status"),
                created_at: chrono::DateTime::parse_from_rfc3339(
                    row.get::<String, _>("created_at").as_str(),
                )?
                .with_timezone(&Utc),
                updated_at: chrono::DateTime::parse_from_rfc3339(
                    row.get::<String, _>("updated_at").as_str(),
                )?
                .with_timezone(&Utc),
            });
        }
        Ok(runs)
    }

    pub async fn list_run_events(&self, run_id: &str) -> Result<Vec<RunEvent>> {
        let rows = sqlx::query(
            "SELECT event_id, run_id, phase, agent, status, message, created_at FROM run_events WHERE run_id = ? ORDER BY event_id ASC"
        )
        .bind(run_id)
        .fetch_all(&self.pool)
        .await?;

        let mut events = Vec::new();
        for row in rows {
            events.push(RunEvent {
                event_id: row.get::<i64, _>("event_id"),
                run_id: row.get::<String, _>("run_id"),
                phase: row.get::<String, _>("phase"),
                agent: row.get::<String, _>("agent"),
                status: row.get::<String, _>("status"),
                message: row.get::<String, _>("message"),
                created_at: chrono::DateTime::parse_from_rfc3339(
                    row.get::<String, _>("created_at").as_str(),
                )?
                .with_timezone(&Utc),
            });
        }
        Ok(events)
    }

    pub async fn package_artifacts(&self, run_id: &str) -> Result<PathBuf> {
        let run = self
            .get_run(run_id)
            .await?
            .with_context(|| format!("run not found: {run_id}"))?;
        let events = self.list_run_events(run_id).await?;
        let bundle_dir = self.paths.artifacts.join(run_id);
        tokio::fs::create_dir_all(&bundle_dir).await?;

        let summary = render_bundle_summary(&run, &events);
        tokio::fs::write(bundle_dir.join("SUMMARY.txt"), summary).await?;
        self.write_json_file(&bundle_dir.join("run.json"), &run).await?;
        self.write_json_file(&bundle_dir.join("events.json"), &events).await?;

        let run_dir = self.run_dir(run_id);
        if run_dir.exists() {
            copy_tree_contents(&run_dir, &run_dir, &bundle_dir)?;
        }

        let log_path = self.run_log_path(run_id);
        if log_path.exists() {
            std::fs::copy(&log_path, bundle_dir.join("run.log"))
                .with_context(|| format!("copying run log {}", log_path.display()))?;
        }

        let staged_product = self.workspace_root(run_id).join("product");
        if staged_product.exists() {
            let manifest = list_relative_files(&staged_product, &staged_product)?;
            tokio::fs::write(
                bundle_dir.join("workspace_manifest.txt"),
                manifest.join("\n"),
            )
            .await?;
        }

        Ok(bundle_dir)
    }
}

fn build_implementation_plan(
    spec_obj: &Spec,
    intent: &IntentPackage,
    memory_hits: &[String],
    staged_product: &Path,
) -> String {
    let memory_summary = if memory_hits.is_empty() {
        "No prior memory hits found.".to_string()
    } else {
        memory_hits.join("\n\n")
    };
    format!(
        "# Mason Implementation Plan\n\nProduct workspace: {}\n\n## Intent\n{}\n\n## Scope\n{}\n\n## Acceptance Criteria\n{}\n\n## Recommended Steps\n{}\n\n## Prior Context\n{}\n",
        staged_product.display(),
        intent.summary,
        spec_obj.scope.join("\n- "),
        spec_obj.acceptance_criteria.join("\n- "),
        intent.recommended_steps.join("\n- "),
        memory_summary
    )
}

fn format_memory_context(memory_hits: &[String]) -> String {
    if memory_hits.is_empty() {
        "No memory hits collected for this run.".to_string()
    } else {
        memory_hits.join("\n\n---\n\n")
    }
}

fn detect_package_scripts(package_json: &Path) -> Result<Vec<String>> {
    let content = std::fs::read_to_string(package_json)
        .with_context(|| format!("reading {}", package_json.display()))?;
    let parsed: serde_json::Value = serde_json::from_str(&content)
        .with_context(|| format!("parsing {}", package_json.display()))?;
    let scripts = parsed
        .get("scripts")
        .and_then(|value| value.as_object())
        .map(|map| map.keys().cloned().collect())
        .unwrap_or_else(Vec::new);
    Ok(scripts)
}

fn command_detail(command: &str, outcome: &CommandOutcome) -> String {
    let output = if !outcome.stderr.is_empty() {
        outcome.stderr.as_str()
    } else {
        outcome.stdout.as_str()
    };
    let excerpt = truncate_text(output, 220);
    format!(
        "{} -> success={} code={:?} {}",
        command, outcome.success, outcome.code, excerpt
    )
}

fn format_command_output(command: &str, outcome: &CommandOutcome) -> String {
    let mut sections = vec![format!("$ {}", command)];
    if !outcome.stdout.is_empty() {
        sections.push(format!("stdout:\n{}", outcome.stdout));
    }
    if !outcome.stderr.is_empty() {
        sections.push(format!("stderr:\n{}", outcome.stderr));
    }
    sections.push(format!("exit_code: {:?}", outcome.code));
    sections.join("\n\n")
}

fn truncate_text(text: &str, max_len: usize) -> String {
    let trimmed = text.trim();
    if trimmed.is_empty() {
        return "(no output)".to_string();
    }
    if trimmed.chars().count() <= max_len {
        trimmed.to_string()
    } else {
        let mut out = trimmed.chars().take(max_len).collect::<String>();
        out.push_str("...");
        out
    }
}

fn list_relative_files(root: &Path, current: &Path) -> Result<Vec<String>> {
    let mut files = Vec::new();
    for entry in std::fs::read_dir(current)
        .with_context(|| format!("reading {}", current.display()))?
    {
        let entry = entry?;
        let path = entry.path();
        let file_type = entry.file_type()?;
        if file_type.is_dir() {
            files.extend(list_relative_files(root, &path)?);
        } else if file_type.is_file() {
            let relative = path
                .strip_prefix(root)
                .with_context(|| format!("stripping prefix {}", root.display()))?;
            files.push(relative.display().to_string());
        }
    }
    files.sort();
    Ok(files)
}

fn list_run_directory(run_dir: &Path) -> Result<Vec<String>> {
    let mut files = list_relative_files(run_dir, run_dir)?;
    files.insert(0, format!("run_dir={}", run_dir.display()));
    Ok(files)
}

fn copy_tree_contents(source_root: &Path, current: &Path, destination_root: &Path) -> Result<()> {
    for entry in std::fs::read_dir(current)
        .with_context(|| format!("reading {}", current.display()))?
    {
        let entry = entry?;
        let path = entry.path();
        let file_type = entry.file_type()?;
        let relative = path
            .strip_prefix(source_root)
            .with_context(|| format!("stripping prefix {}", source_root.display()))?;
        let destination = destination_root.join(relative);
        if file_type.is_dir() {
            std::fs::create_dir_all(&destination)
                .with_context(|| format!("creating {}", destination.display()))?;
            copy_tree_contents(source_root, &path, destination_root)?;
        } else if file_type.is_file() {
            if let Some(parent) = destination.parent() {
                std::fs::create_dir_all(parent)
                    .with_context(|| format!("creating {}", parent.display()))?;
            }
            std::fs::copy(&path, &destination).with_context(|| {
                format!("copying {} -> {}", path.display(), destination.display())
            })?;
        }
    }
    Ok(())
}

fn render_bundle_summary(run: &RunRecord, events: &[RunEvent]) -> String {
    let mut lines = vec![
        "Run Bundle".to_string(),
        "==========".to_string(),
        format!("Run ID: {}", run.run_id),
        format!("Spec ID: {}", run.spec_id),
        format!("Product: {}", run.product),
        format!("Status: {}", run.status),
        format!("Created: {}", run.created_at),
        format!("Updated: {}", run.updated_at),
        String::new(),
        "Timeline".to_string(),
        "--------".to_string(),
    ];

    if events.is_empty() {
        lines.push("No events recorded".to_string());
    } else {
        for event in events {
            lines.push(format!(
                "{} [{}] {} {} - {}",
                event.created_at, event.phase, event.agent, event.status, event.message
            ));
        }
    }

    lines.join("\n") + "\n"
}
