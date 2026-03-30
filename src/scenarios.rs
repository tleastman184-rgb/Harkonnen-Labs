use anyhow::{Context, Result};
use serde::Deserialize;
use std::path::Path;

use crate::models::{
    AgentExecution, HiddenScenarioCheckResult, HiddenScenarioEvaluation, HiddenScenarioSummary,
    RunEvent, TwinEnvironment, ValidationSummary,
};

#[derive(Debug, Clone, Deserialize)]
pub struct HiddenScenarioFile {
    pub id: String,
    pub spec_id: String,
    pub title: String,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub checks: Vec<HiddenScenarioCheck>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum HiddenScenarioCheck {
    RunStatus { equals: String },
    EventPresent { phase: String, agent: String },
    ArtifactExists { path: String },
    ValidationPassed,
    AgentExecuted { agent: String },
    TwinServicePresent { service: String },
}

pub fn load_hidden_scenarios(root: &Path, spec_id: &str) -> Result<Vec<HiddenScenarioFile>> {
    let mut scenarios = Vec::new();
    if !root.exists() {
        return Ok(scenarios);
    }

    collect_scenarios(root, spec_id, &mut scenarios)?;
    scenarios.sort_by(|left, right| left.id.cmp(&right.id));
    Ok(scenarios)
}

fn collect_scenarios(
    current: &Path,
    spec_id: &str,
    scenarios: &mut Vec<HiddenScenarioFile>,
) -> Result<()> {
    for entry in std::fs::read_dir(current)
        .with_context(|| format!("reading scenario directory {}", current.display()))?
    {
        let entry = entry?;
        let path = entry.path();
        let file_type = entry.file_type()?;
        if file_type.is_dir() {
            collect_scenarios(&path, spec_id, scenarios)?;
            continue;
        }
        if path.extension().and_then(|value| value.to_str()) != Some("yaml") {
            continue;
        }
        let raw = std::fs::read_to_string(&path)
            .with_context(|| format!("reading hidden scenario {}", path.display()))?;
        let scenario: HiddenScenarioFile = serde_yaml::from_str(&raw)
            .with_context(|| format!("parsing hidden scenario {}", path.display()))?;
        if scenario.spec_id == spec_id {
            scenarios.push(scenario);
        }
    }
    Ok(())
}

pub fn evaluate_hidden_scenarios(
    scenarios: &[HiddenScenarioFile],
    run_status: &str,
    events: &[RunEvent],
    validation: &ValidationSummary,
    twin: &TwinEnvironment,
    agent_executions: &[AgentExecution],
    run_dir: &Path,
) -> HiddenScenarioSummary {
    if scenarios.is_empty() {
        return HiddenScenarioSummary {
            passed: false,
            results: vec![HiddenScenarioEvaluation {
                scenario_id: "scenario_store_empty".to_string(),
                title: "No hidden scenarios available".to_string(),
                passed: false,
                details: "No hidden scenario definitions matched this spec.".to_string(),
                checks: vec![],
            }],
        };
    }

    let results = scenarios
        .iter()
        .map(|scenario| {
            let checks = scenario
                .checks
                .iter()
                .map(|check| evaluate_check(check, run_status, events, validation, twin, agent_executions, run_dir))
                .collect::<Vec<_>>();
            let passed = checks.iter().all(|check| check.passed);
            HiddenScenarioEvaluation {
                scenario_id: scenario.id.clone(),
                title: scenario.title.clone(),
                passed,
                details: if passed {
                    scenario.description.clone()
                } else {
                    format!("{} (one or more hidden checks failed)", scenario.description)
                },
                checks,
            }
        })
        .collect::<Vec<_>>();

    HiddenScenarioSummary {
        passed: results.iter().all(|result| result.passed),
        results,
    }
}

fn evaluate_check(
    check: &HiddenScenarioCheck,
    run_status: &str,
    events: &[RunEvent],
    validation: &ValidationSummary,
    twin: &TwinEnvironment,
    agent_executions: &[AgentExecution],
    run_dir: &Path,
) -> HiddenScenarioCheckResult {
    match check {
        HiddenScenarioCheck::RunStatus { equals } => HiddenScenarioCheckResult {
            kind: format!("run_status == {equals}"),
            passed: run_status == equals,
            details: format!("actual status: {run_status}"),
        },
        HiddenScenarioCheck::EventPresent { phase, agent } => {
            let found = events
                .iter()
                .any(|event| event.phase == *phase && event.agent == *agent);
            HiddenScenarioCheckResult {
                kind: format!("event_present({phase}, {agent})"),
                passed: found,
                details: if found {
                    format!("found event for {agent} during {phase}")
                } else {
                    format!("missing event for {agent} during {phase}")
                },
            }
        }
        HiddenScenarioCheck::ArtifactExists { path } => {
            let target = run_dir.join(path);
            HiddenScenarioCheckResult {
                kind: format!("artifact_exists({path})"),
                passed: target.exists(),
                details: target.display().to_string(),
            }
        }
        HiddenScenarioCheck::ValidationPassed => HiddenScenarioCheckResult {
            kind: "validation_passed".to_string(),
            passed: validation.passed,
            details: format!("visible validation passed: {}", validation.passed),
        },
        HiddenScenarioCheck::AgentExecuted { agent } => {
            let found = agent_executions
                .iter()
                .any(|execution| execution.agent_name == *agent);
            HiddenScenarioCheckResult {
                kind: format!("agent_executed({agent})"),
                passed: found,
                details: if found {
                    format!("found execution transcript for {agent}")
                } else {
                    format!("missing execution transcript for {agent}")
                },
            }
        }
        HiddenScenarioCheck::TwinServicePresent { service } => {
            let found = twin.services.iter().any(|candidate| candidate.name == *service);
            HiddenScenarioCheckResult {
                kind: format!("twin_service_present({service})"),
                passed: found,
                details: if found {
                    format!("twin includes service {service}")
                } else {
                    format!("twin does not include service {service}")
                },
            }
        }
    }
}
