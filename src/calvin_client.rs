use anyhow::{Context, Result};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::time::Duration;

use crate::setup::CalvinConfig;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArchiveExperience {
    pub run_id: String,
    pub episode_id: Option<String>,
    pub provider: String,
    pub model: String,
    pub narrative_summary: String,
    pub scope: String,
    pub chamber: Chamber,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Chamber {
    Mythos,
    Episteme,
    Ethos,
    Pathos,
    Logos,
    Praxis,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RevisionType {
    FastLoop,
    MediumLoop,
    SchemaRevision,
    PolicyChange,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BeliefRevision {
    pub belief_id: String,
    pub prior_confidence: f64,
    pub revised_summary: String,
    pub new_confidence: f64,
    pub revision_reason: String,
    pub evidence_ids: Vec<String>,
    pub revision_type: RevisionType,
    pub preservation_note: Option<String>,
}

impl BeliefRevision {
    pub fn validate(&self) -> Result<()> {
        validate_confidence("prior_confidence", self.prior_confidence)?;
        validate_confidence("new_confidence", self.new_confidence)?;
        if self.evidence_ids.is_empty() {
            anyhow::bail!("BeliefRevision requires at least one evidence_id");
        }
        let min_evidence = match self.revision_type {
            RevisionType::FastLoop | RevisionType::PolicyChange => 1,
            RevisionType::MediumLoop | RevisionType::SchemaRevision => 3,
        };
        if self.evidence_ids.len() < min_evidence {
            anyhow::bail!(
                "{:?} BeliefRevision requires at least {} evidence_ids",
                self.revision_type,
                min_evidence
            );
        }
        Ok(())
    }
}

fn validate_confidence(label: &str, value: f64) -> Result<()> {
    if value.is_finite() && (0.0..=1.0).contains(&value) {
        Ok(())
    } else {
        anyhow::bail!("{label} must be finite and in 0.0..=1.0")
    }
}

/// Coobie's pre-run epistemic claim about what will happen in this run.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RunPrediction {
    pub prediction_id: String,
    pub run_id: String,
    pub spec_id: String,
    /// "pass" | "fail" | "uncertain"
    pub predicted_outcome: String,
    /// 0.0 = low risk, 1.0 = very likely to fail
    pub risk_score: f64,
    /// How confident Coobie is in this prediction.
    pub confidence: f64,
    /// The phase most likely to fail, if identifiable.
    pub failure_phase: Option<String>,
    /// The failure kind most likely, if identifiable.
    pub failure_kind: Option<String>,
    /// Comma-separated PriorCauseSignal ids that drove this prediction.
    pub source_cause_ids: String,
    pub narrative_summary: String,
}

/// The actual run outcome, linked back to the prediction for error computation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PredictionOutcome {
    pub prediction_id: String,
    pub result_id: String,
    pub run_id: String,
    /// "completed" | "failed" | "completed_with_issues"
    pub actual_outcome: String,
    pub actual_failure_phase: Option<String>,
    pub actual_failure_kind: Option<String>,
    /// 0.0 = prediction was correct, 1.0 = completely wrong.
    pub prediction_error: f64,
    pub narrative_summary: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TelemetryEvent {
    pub agent_id: String,
    pub run_id: String,
    pub phase: Option<String>,
    pub action_type: String,
    pub provider: Option<String>,
    pub model: Option<String>,
    pub outcome: String,
    pub latency_ms: Option<i32>,
    pub tokens_in: Option<i32>,
    pub tokens_out: Option<i32>,
    pub drift_score: Option<f64>,
    pub lab_ness_score: Option<f64>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct MetricsSnapshot {
    pub agent_id: String,
    pub d_star: f64,
    pub ssa: f64,
    pub stress: f64,
    pub hysteresis: Option<f64>,
}

#[derive(Debug, Clone)]
pub struct CalvinClient {
    base_url: String,
    health_client: Client,
    read_client: Client,
    write_client: Client,
}

impl CalvinClient {
    pub fn new(config: &CalvinConfig) -> Result<Self> {
        let health_client = Client::builder()
            .timeout(Duration::from_millis(config.health_timeout_ms))
            .build()
            .context("building CalvinClient health client")?;
        let read_client = Client::builder()
            .timeout(Duration::from_millis(config.read_timeout_ms))
            .build()
            .context("building CalvinClient read client")?;
        let write_client = Client::builder()
            .timeout(Duration::from_millis(config.write_timeout_ms))
            .build()
            .context("building CalvinClient write client")?;
        Ok(Self {
            base_url: config.harmony_url.trim_end_matches('/').to_string(),
            health_client,
            read_client,
            write_client,
        })
    }

    pub async fn health_check(&self) -> bool {
        self.health_client
            .get(format!("{}/health", self.base_url))
            .send()
            .await
            .map(|r| r.status().is_success())
            .unwrap_or(false)
    }

    pub async fn status(&self) -> Result<serde_json::Value> {
        let resp = self
            .read_client
            .get(format!("{}/status", self.base_url))
            .send()
            .await
            .context("GET /status")?;
        Ok(resp.json().await?)
    }

    pub async fn open_run(
        &self,
        run_id: &str,
        spec_id: &str,
        provider: &str,
        model: &str,
    ) -> Result<()> {
        let body = serde_json::json!({
            "run_id": run_id,
            "spec_id": spec_id,
            "provider": provider,
            "model": model,
        });
        self.write_client
            .post(format!("{}/runs", self.base_url))
            .json(&body)
            .send()
            .await
            .context("POST /runs")?;
        Ok(())
    }

    pub async fn record_experience(&self, run_id: &str, exp: &ArchiveExperience) -> Result<()> {
        self.write_client
            .post(format!("{}/runs/{run_id}/experiences", self.base_url))
            .json(exp)
            .send()
            .await
            .context("POST /runs/{run_id}/experiences")?;
        Ok(())
    }

    pub async fn revise_belief(&self, run_id: &str, rev: &BeliefRevision) -> Result<()> {
        rev.validate()?;
        self.write_client
            .post(format!("{}/runs/{run_id}/beliefs", self.base_url))
            .json(rev)
            .send()
            .await
            .context("POST /runs/{run_id}/beliefs")?;
        Ok(())
    }

    pub async fn close_run(&self, run_id: &str, outcome: &str) -> Result<()> {
        let body = serde_json::json!({"outcome": outcome});
        self.write_client
            .patch(format!("{}/runs/{run_id}/close", self.base_url))
            .json(&body)
            .send()
            .await
            .context("PATCH /runs/{run_id}/close")?;
        Ok(())
    }

    pub async fn record_prediction(&self, pred: &RunPrediction) -> Result<()> {
        self.write_client
            .post(format!(
                "{}/runs/{}/predictions",
                self.base_url, pred.run_id
            ))
            .json(&serde_json::json!({
                "prediction_id": pred.prediction_id,
                "predicted_outcome": pred.predicted_outcome,
                "risk_score": pred.risk_score,
                "confidence": pred.confidence,
                "failure_phase": pred.failure_phase,
                "failure_kind": pred.failure_kind,
                "source_cause_ids": pred.source_cause_ids,
                "narrative_summary": pred.narrative_summary,
            }))
            .send()
            .await
            .context("POST /runs/{run_id}/predictions")?;
        Ok(())
    }

    pub async fn record_prediction_result(&self, outcome: &PredictionOutcome) -> Result<()> {
        self.write_client
            .post(format!(
                "{}/runs/{}/prediction-result",
                self.base_url, outcome.run_id
            ))
            .json(&serde_json::json!({
                "prediction_id": outcome.prediction_id,
                "result_id": outcome.result_id,
                "actual_outcome": outcome.actual_outcome,
                "actual_failure_phase": outcome.actual_failure_phase,
                "actual_failure_kind": outcome.actual_failure_kind,
                "prediction_error": outcome.prediction_error,
                "narrative_summary": outcome.narrative_summary,
            }))
            .send()
            .await
            .context("POST /runs/{run_id}/prediction-result")?;
        Ok(())
    }

    pub async fn get_prediction(&self, run_id: &str) -> Result<Option<serde_json::Value>> {
        let resp = self
            .read_client
            .get(format!("{}/runs/{run_id}/predictions", self.base_url))
            .send()
            .await
            .context("GET /runs/{run_id}/predictions")?;
        if resp.status() == reqwest::StatusCode::NOT_FOUND {
            return Ok(None);
        }
        Ok(Some(resp.json().await?))
    }

    pub async fn record_causal_link(
        &self,
        run_id: &str,
        cause_episode_id: &str,
        effect_episode_id: &str,
        pearl_level: &str,
        confidence: f64,
    ) -> Result<()> {
        self.write_client
            .post(format!("{}/runs/{run_id}/causal-links", self.base_url))
            .json(&serde_json::json!({
                "cause_episode_id": cause_episode_id,
                "effect_episode_id": effect_episode_id,
                "pearl_level": pearl_level,
                "confidence": confidence,
            }))
            .send()
            .await
            .context("POST /runs/{run_id}/causal-links")?;
        Ok(())
    }

    pub async fn update_agent_status(&self, agent_name: &str, status: &str) -> Result<()> {
        self.write_client
            .patch(format!("{}/agents/{agent_name}/status", self.base_url))
            .json(&serde_json::json!({"status": status}))
            .send()
            .await
            .context("PATCH /agents/{agent_name}/status")?;
        Ok(())
    }

    pub async fn get_kernel_traits(&self, agent_name: &str) -> Result<Vec<String>> {
        let resp = self
            .read_client
            .get(format!("{}/agents/{agent_name}/traits", self.base_url))
            .send()
            .await
            .context("GET /agents/{agent_name}/traits")?;
        Ok(resp.json().await?)
    }

    pub async fn get_active_beliefs(&self, agent_name: &str) -> Result<Vec<String>> {
        let resp = self
            .read_client
            .get(format!("{}/agents/{agent_name}/beliefs", self.base_url))
            .send()
            .await
            .context("GET /agents/{agent_name}/beliefs")?;
        Ok(resp.json().await?)
    }

    pub async fn check_adaptation_safe(
        &self,
        agent_name: &str,
        adaptation_summary: &str,
    ) -> Result<bool> {
        let body = serde_json::json!({"adaptation_summary": adaptation_summary});
        let resp = self
            .read_client
            .post(format!("{}/agents/{agent_name}/check", self.base_url))
            .json(&body)
            .send()
            .await
            .context("POST /agents/{agent_name}/check")?;
        let v: serde_json::Value = resp.json().await?;
        Ok(v["safe"].as_bool().unwrap_or(true))
    }

    pub async fn get_metrics(&self, agent_name: &str) -> Result<MetricsSnapshot> {
        let resp = self
            .read_client
            .get(format!("{}/agents/{agent_name}/metrics", self.base_url))
            .send()
            .await
            .context("GET /agents/{agent_name}/metrics")?;
        Ok(resp.json().await?)
    }

    pub async fn write_event(&self, evt: &TelemetryEvent) -> Result<()> {
        self.write_client
            .post(format!("{}/telemetry", self.base_url))
            .json(evt)
            .send()
            .await
            .context("POST /telemetry")?;
        Ok(())
    }

    pub async fn write_events_batch(&self, evts: &[TelemetryEvent]) -> Result<()> {
        self.write_client
            .post(format!("{}/telemetry/batch", self.base_url))
            .json(&evts)
            .send()
            .await
            .context("POST /telemetry/batch")?;
        Ok(())
    }
}

/// Try to create a CalvinClient; returns None with a warning if disabled or unreachable.
pub async fn try_connect(config: &CalvinConfig) -> Option<CalvinClient> {
    if !config.enabled {
        return None;
    }
    match CalvinClient::new(config) {
        Ok(client) => {
            if client.health_check().await {
                Some(client)
            } else {
                tracing::warn!(
                    "Calvin Archive enabled but harmony is not responding at {}",
                    config.harmony_url
                );
                None
            }
        }
        Err(e) => {
            tracing::warn!("Calvin Archive disabled — client init failed: {e}");
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{BeliefRevision, CalvinClient, RevisionType};
    use crate::setup::CalvinConfig;

    fn revision(revision_type: RevisionType, evidence_ids: Vec<&str>) -> BeliefRevision {
        BeliefRevision {
            belief_id: "belief-1".to_string(),
            prior_confidence: 0.4,
            revised_summary: "Updated belief".to_string(),
            new_confidence: 0.8,
            revision_reason: "New evidence arrived.".to_string(),
            evidence_ids: evidence_ids.into_iter().map(str::to_string).collect(),
            revision_type,
            preservation_note: None,
        }
    }

    #[test]
    fn client_builds_with_independent_timeout_profile() {
        let config = CalvinConfig {
            health_timeout_ms: 250,
            read_timeout_ms: 1_500,
            write_timeout_ms: 7_500,
            ..CalvinConfig::default()
        };

        assert!(CalvinClient::new(&config).is_ok());
    }

    #[test]
    fn belief_revision_requires_computable_confidence_and_evidence() {
        assert!(revision(RevisionType::FastLoop, vec!["experience-1"])
            .validate()
            .is_ok());

        let mut missing_prior = revision(RevisionType::FastLoop, vec!["experience-1"]);
        missing_prior.prior_confidence = f64::NAN;
        assert!(missing_prior.validate().is_err());

        assert!(revision(RevisionType::FastLoop, Vec::new())
            .validate()
            .is_err());
    }

    #[test]
    fn medium_loop_and_schema_revision_require_cross_episode_evidence() {
        assert!(revision(RevisionType::MediumLoop, vec!["e1", "e2"])
            .validate()
            .is_err());
        assert!(revision(RevisionType::MediumLoop, vec!["e1", "e2", "e3"])
            .validate()
            .is_ok());
        assert!(revision(RevisionType::SchemaRevision, vec!["e1", "e2"])
            .validate()
            .is_err());
        assert!(
            revision(RevisionType::SchemaRevision, vec!["e1", "e2", "e3"])
                .validate()
                .is_ok()
        );
    }
}
