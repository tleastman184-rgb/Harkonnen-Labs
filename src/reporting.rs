use anyhow::Result;
use serde::de::DeserializeOwned;
use std::path::Path;

use crate::{
    models::{AgentExecution, HiddenScenarioSummary, TwinEnvironment, ValidationSummary},
    orchestrator::AppContext,
};

pub async fn build_report(app: &AppContext, run_id: &str) -> Result<String> {
    let run = app.get_run(run_id).await?;
    let Some(run) = run else {
        return Ok(format!("Run not found: {run_id}"));
    };

    let events = app.list_run_events(run_id).await?;
    let run_dir = app.paths.workspaces.join(run_id).join("run");
    let validation: Option<ValidationSummary> = read_optional_json(&run_dir.join("validation.json")).await?;
    let twin: Option<TwinEnvironment> = read_optional_json(&run_dir.join("twin.json")).await?;
    let hidden: Option<HiddenScenarioSummary> =
        read_optional_json(&run_dir.join("hidden_scenarios.json")).await?;
    let agent_executions: Option<Vec<AgentExecution>> =
        read_optional_json(&run_dir.join("agent_executions.json")).await?;

    let mut report = format!(
        "Run Report\n==========\nRun ID: {}\nSpec ID: {}\nProduct: {}\nStatus: {}\nCreated: {}\nUpdated: {}\nWorkspace: {}\nArtifacts: {}\n",
        run.run_id,
        run.spec_id,
        run.product,
        run.status,
        run.created_at,
        run.updated_at,
        app.paths.workspaces.join(run_id).display(),
        app.paths.artifacts.join(run_id).display(),
    );

    report.push_str("\nTimeline\n--------\n");
    if events.is_empty() {
        report.push_str("No run events recorded.\n");
    } else {
        for event in &events {
            report.push_str(&format!(
                "{} [{}] {} {} - {}\n",
                event.created_at, event.phase, event.agent, event.status, event.message
            ));
        }
    }

    report.push_str("\nAgents\n------\n");
    if let Some(agent_executions) = agent_executions {
        for execution in agent_executions {
            report.push_str(&format!(
                "- {} -> {} / {} / mode={}\n  {}\n",
                execution.agent_name,
                execution.provider,
                execution.model,
                execution.mode,
                execution.summary
            ));
        }
    } else {
        report.push_str("No agent execution transcripts written yet.\n");
    }

    report.push_str("\nTwin Environment\n----------------\n");
    if let Some(twin) = twin {
        report.push_str(&format!("Status: {}\n", twin.status));
        for service in twin.services {
            report.push_str(&format!(
                "- {} [{}] {}\n  {}\n",
                service.name, service.kind, service.status, service.details
            ));
        }
    } else {
        report.push_str("No twin environment manifest written yet.\n");
    }

    report.push_str("\nVisible Validation\n------------------\n");
    if let Some(validation) = validation {
        report.push_str(&format!("Passed: {}\n", validation.passed));
        for result in validation.results {
            report.push_str(&format!(
                "- {}: {}\n  {}\n",
                result.scenario_id,
                if result.passed { "pass" } else { "fail" },
                result.details
            ));
        }
    } else {
        report.push_str("No validation summary written yet.\n");
    }

    report.push_str("\nHidden Scenarios\n----------------\n");
    if let Some(hidden) = hidden {
        report.push_str(&format!("Passed: {}\n", hidden.passed));
        for result in hidden.results {
            report.push_str(&format!(
                "- {}: {}\n  {}\n",
                result.scenario_id,
                if result.passed { "pass" } else { "fail" },
                result.details
            ));
            for check in result.checks {
                report.push_str(&format!(
                    "    * {} -> {} ({})\n",
                    check.kind,
                    if check.passed { "pass" } else { "fail" },
                    check.details
                ));
            }
        }
    } else {
        report.push_str("No hidden scenario summary written yet.\n");
    }

    Ok(report)
}

async fn read_optional_json<T: DeserializeOwned>(path: &Path) -> Result<Option<T>> {
    if !path.exists() {
        return Ok(None);
    }
    let raw = tokio::fs::read_to_string(path).await?;
    Ok(Some(serde_json::from_str::<T>(&raw)?))
}
