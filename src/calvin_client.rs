use anyhow::{Context, Result};
use chrono::{Duration as ChronoDuration, Utc};
use reqwest::{Client, Method};
use serde::{Deserialize, Serialize};
use sqlx::{Row, SqlitePool};
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

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum PearlLevel {
    #[serde(alias = "Associational")]
    Associational,
    #[serde(alias = "Interventional")]
    Interventional,
    #[serde(alias = "Counterfactual")]
    Counterfactual,
}

impl PearlLevel {
    fn as_label(&self) -> &'static str {
        match self {
            Self::Associational => "Associational",
            Self::Interventional => "Interventional",
            Self::Counterfactual => "Counterfactual",
        }
    }

    fn from_label(value: &str) -> Self {
        match value.to_ascii_lowercase().as_str() {
            "interventional" => Self::Interventional,
            "counterfactual" => Self::Counterfactual,
            _ => Self::Associational,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CausalLinkPayload {
    pub cause_episode_id: String,
    pub effect_episode_id: String,
    pub pearl_level: PearlLevel,
    pub confidence: f64,
    #[serde(default)]
    pub held_fixed: Vec<String>,
    pub estimated_effect_delta: Option<f64>,
    pub actual_trace_id: Option<String>,
    pub hypothetical_intervention: Option<String>,
    #[serde(default)]
    pub warrant_gap: bool,
    pub epistemic_warrant: Option<PearlLevel>,
}

impl CausalLinkPayload {
    pub fn associational(cause_episode_id: &str, effect_episode_id: &str, confidence: f64) -> Self {
        Self {
            cause_episode_id: cause_episode_id.to_string(),
            effect_episode_id: effect_episode_id.to_string(),
            pearl_level: PearlLevel::Associational,
            confidence,
            held_fixed: Vec::new(),
            estimated_effect_delta: None,
            actual_trace_id: None,
            hypothetical_intervention: None,
            warrant_gap: false,
            epistemic_warrant: Some(PearlLevel::Associational),
        }
    }

    pub fn normalized(mut self) -> Result<Self> {
        validate_confidence("causal_link.confidence", self.confidence)?;
        if self.cause_episode_id.trim().is_empty() {
            anyhow::bail!("CausalLinkPayload requires cause_episode_id");
        }
        if self.effect_episode_id.trim().is_empty() {
            anyhow::bail!("CausalLinkPayload requires effect_episode_id");
        }
        let requested = self.pearl_level.clone();
        let warranted = if self.has_counterfactual_fields() {
            PearlLevel::Counterfactual
        } else if self.has_interventional_fields() {
            PearlLevel::Interventional
        } else {
            PearlLevel::Associational
        };
        if pearl_rank(&warranted) < pearl_rank(&requested) {
            self.pearl_level = warranted.clone();
            self.warrant_gap = true;
        }
        self.epistemic_warrant = Some(warranted);
        Ok(self)
    }

    fn has_interventional_fields(&self) -> bool {
        !self.held_fixed.is_empty() && self.estimated_effect_delta.is_some()
    }

    fn has_counterfactual_fields(&self) -> bool {
        self.has_interventional_fields()
            && self
                .actual_trace_id
                .as_deref()
                .map(|value| !value.trim().is_empty())
                .unwrap_or(false)
            && self
                .hypothetical_intervention
                .as_deref()
                .map(|value| !value.trim().is_empty())
                .unwrap_or(false)
    }
}

fn pearl_rank(level: &PearlLevel) -> u8 {
    match level {
        PearlLevel::Associational => 0,
        PearlLevel::Interventional => 1,
        PearlLevel::Counterfactual => 2,
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct MetricsSnapshot {
    pub agent_id: String,
    pub d_star: f64,
    pub ssa: f64,
    pub stress: f64,
    pub hysteresis: Option<f64>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct CalvinWriteQueueStats {
    pub pending: i64,
    pub retry_pending: i64,
    pub confirmed: i64,
    pub oldest_pending_at: Option<String>,
    pub next_attempt_at: Option<String>,
    pub last_error: Option<String>,
}

impl CalvinWriteQueueStats {
    pub fn pending_confirmation_count(&self) -> i64 {
        self.pending + self.retry_pending
    }
}

#[derive(Debug, Clone)]
pub struct CalvinClient {
    base_url: String,
    health_client: Client,
    read_client: Client,
    write_client: Client,
    queue_pool: Option<SqlitePool>,
}

impl CalvinClient {
    pub fn new(config: &CalvinConfig) -> Result<Self> {
        Self::new_with_queue(config, None)
    }

    pub fn new_with_pool(config: &CalvinConfig, queue_pool: SqlitePool) -> Result<Self> {
        Self::new_with_queue(config, Some(queue_pool))
    }

    fn new_with_queue(config: &CalvinConfig, queue_pool: Option<SqlitePool>) -> Result<Self> {
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
            queue_pool,
        })
    }

    pub fn spawn_write_queue_processor(&self) {
        let Some(pool) = self.queue_pool.clone() else {
            return;
        };
        let client = self.write_client.clone();
        let base_url = self.base_url.clone();
        tokio::spawn(async move {
            loop {
                if let Err(error) = drain_calvin_write_queue_once(&pool, &client, &base_url).await {
                    tracing::warn!(error = %error, "Calvin write queue drain failed");
                }
                tokio::time::sleep(Duration::from_secs(2)).await;
            }
        });
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
        self.enqueue_or_send_write("open_run", Some(run_id), Method::POST, "/runs", body)
            .await
            .context("POST /runs")?;
        Ok(())
    }

    pub async fn record_experience(&self, run_id: &str, exp: &ArchiveExperience) -> Result<()> {
        let path = format!("/runs/{run_id}/experiences");
        let body = serde_json::to_value(exp)?;
        self.enqueue_or_send_write("record_experience", Some(run_id), Method::POST, &path, body)
            .await
            .context("POST /runs/{run_id}/experiences")?;
        Ok(())
    }

    pub async fn revise_belief(&self, run_id: &str, rev: &BeliefRevision) -> Result<()> {
        rev.validate()?;
        let path = format!("/runs/{run_id}/beliefs");
        let body = serde_json::to_value(rev)?;
        self.enqueue_or_send_write("revise_belief", Some(run_id), Method::POST, &path, body)
            .await
            .context("POST /runs/{run_id}/beliefs")?;
        Ok(())
    }

    pub async fn close_run(&self, run_id: &str, outcome: &str) -> Result<()> {
        let body = serde_json::json!({"outcome": outcome});
        let path = format!("/runs/{run_id}/close");
        self.enqueue_or_send_write("close_run", Some(run_id), Method::PATCH, &path, body)
            .await
            .context("PATCH /runs/{run_id}/close")?;
        Ok(())
    }

    pub async fn record_prediction(&self, pred: &RunPrediction) -> Result<()> {
        let path = format!("/runs/{}/predictions", pred.run_id);
        let body = serde_json::json!({
                "prediction_id": pred.prediction_id,
                "predicted_outcome": pred.predicted_outcome,
                "risk_score": pred.risk_score,
                "confidence": pred.confidence,
                "failure_phase": pred.failure_phase,
                "failure_kind": pred.failure_kind,
                "source_cause_ids": pred.source_cause_ids,
                "narrative_summary": pred.narrative_summary,
        });
        self.enqueue_or_send_write(
            "record_prediction",
            Some(&pred.run_id),
            Method::POST,
            &path,
            body,
        )
        .await
        .context("POST /runs/{run_id}/predictions")?;
        Ok(())
    }

    pub async fn record_prediction_result(&self, outcome: &PredictionOutcome) -> Result<()> {
        let path = format!("/runs/{}/prediction-result", outcome.run_id);
        let body = serde_json::json!({
                "prediction_id": outcome.prediction_id,
                "result_id": outcome.result_id,
                "actual_outcome": outcome.actual_outcome,
                "actual_failure_phase": outcome.actual_failure_phase,
                "actual_failure_kind": outcome.actual_failure_kind,
                "prediction_error": outcome.prediction_error,
                "narrative_summary": outcome.narrative_summary,
        });
        self.enqueue_or_send_write(
            "record_prediction_result",
            Some(&outcome.run_id),
            Method::POST,
            &path,
            body,
        )
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
        let mut payload =
            CausalLinkPayload::associational(cause_episode_id, effect_episode_id, confidence);
        payload.pearl_level = PearlLevel::from_label(pearl_level);
        self.record_structural_causal_link(run_id, payload).await
    }

    pub async fn record_structural_causal_link(
        &self,
        run_id: &str,
        payload: CausalLinkPayload,
    ) -> Result<()> {
        let path = format!("/runs/{run_id}/causal-links");
        let payload = payload.normalized()?;
        let body = serde_json::json!({
            "cause_episode_id": payload.cause_episode_id,
            "effect_episode_id": payload.effect_episode_id,
            "pearl_level": payload.pearl_level.as_label(),
            "confidence": payload.confidence,
            "held_fixed": payload.held_fixed,
            "estimated_effect_delta": payload.estimated_effect_delta,
            "actual_trace_id": payload.actual_trace_id,
            "hypothetical_intervention": payload.hypothetical_intervention,
            "epistemic_warrant": payload.epistemic_warrant.as_ref().map(PearlLevel::as_label),
            "warrant_gap": payload.warrant_gap,
        });
        self.enqueue_or_send_write(
            "record_causal_link",
            Some(run_id),
            Method::POST,
            &path,
            body,
        )
        .await
        .context("POST /runs/{run_id}/causal-links")?;
        Ok(())
    }

    pub async fn update_agent_status(&self, agent_name: &str, status: &str) -> Result<()> {
        let path = format!("/agents/{agent_name}/status");
        let body = serde_json::json!({"status": status});
        self.enqueue_or_send_write("update_agent_status", None, Method::PATCH, &path, body)
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
        let body = serde_json::to_value(evt)?;
        self.enqueue_or_send_write(
            "write_event",
            Some(&evt.run_id),
            Method::POST,
            "/telemetry",
            body,
        )
        .await
        .context("POST /telemetry")?;
        Ok(())
    }

    pub async fn write_events_batch(&self, evts: &[TelemetryEvent]) -> Result<()> {
        let run_id = evts.first().map(|evt| evt.run_id.as_str());
        let body = serde_json::to_value(evts)?;
        self.enqueue_or_send_write(
            "write_events_batch",
            run_id,
            Method::POST,
            "/telemetry/batch",
            body,
        )
        .await
        .context("POST /telemetry/batch")?;
        Ok(())
    }

    async fn enqueue_or_send_write(
        &self,
        operation: &str,
        run_id: Option<&str>,
        method: Method,
        path: &str,
        payload: serde_json::Value,
    ) -> Result<()> {
        if let Some(pool) = self.queue_pool.as_ref() {
            enqueue_calvin_write(pool, operation, run_id, &method, path, &payload).await
        } else {
            send_calvin_write(&self.write_client, &self.base_url, method, path, &payload).await
        }
    }
}

async fn enqueue_calvin_write(
    pool: &SqlitePool,
    operation: &str,
    run_id: Option<&str>,
    method: &Method,
    path: &str,
    payload: &serde_json::Value,
) -> Result<()> {
    let now = Utc::now().to_rfc3339();
    sqlx::query(
        r#"
        INSERT INTO calvin_write_queue
            (queue_id, run_id, operation, method, path, payload_json, status,
             attempts, next_attempt_at, created_at, updated_at)
        VALUES (?1, ?2, ?3, ?4, ?5, ?6, 'pending', 0, ?7, ?7, ?7)
        "#,
    )
    .bind(uuid::Uuid::new_v4().to_string())
    .bind(run_id)
    .bind(operation)
    .bind(method.as_str())
    .bind(path)
    .bind(serde_json::to_string(payload)?)
    .bind(now)
    .execute(pool)
    .await
    .context("enqueueing Calvin archive write")?;
    Ok(())
}

async fn drain_calvin_write_queue_once(
    pool: &SqlitePool,
    client: &Client,
    base_url: &str,
) -> Result<usize> {
    let now = Utc::now().to_rfc3339();
    let rows = sqlx::query(
        r#"
        SELECT queue_id, method, path, payload_json, attempts
        FROM calvin_write_queue
        WHERE status IN ('pending', 'retry_pending')
          AND next_attempt_at <= ?1
        ORDER BY created_at ASC
        LIMIT 25
        "#,
    )
    .bind(&now)
    .fetch_all(pool)
    .await
    .context("loading Calvin write queue")?;

    let mut processed = 0usize;
    for row in rows {
        let queue_id: String = row.get("queue_id");
        let method_raw: String = row.get("method");
        let path: String = row.get("path");
        let payload_json: String = row.get("payload_json");
        let attempts: i64 = row.get("attempts");
        let method = match Method::from_bytes(method_raw.as_bytes()) {
            Ok(method) => method,
            Err(error) => {
                mark_calvin_write_retry(
                    pool,
                    &queue_id,
                    attempts,
                    format!("invalid HTTP method in queue row: {error}"),
                )
                .await?;
                continue;
            }
        };
        let payload = match serde_json::from_str::<serde_json::Value>(&payload_json) {
            Ok(payload) => payload,
            Err(error) => {
                mark_calvin_write_retry(
                    pool,
                    &queue_id,
                    attempts,
                    format!("invalid JSON payload in queue row: {error}"),
                )
                .await?;
                continue;
            }
        };

        match send_calvin_write(client, base_url, method, &path, &payload).await {
            Ok(()) => {
                mark_calvin_write_confirmed(pool, &queue_id).await?;
                processed += 1;
            }
            Err(error) => {
                mark_calvin_write_retry(pool, &queue_id, attempts, error.to_string()).await?;
            }
        }
    }

    Ok(processed)
}

async fn send_calvin_write(
    client: &Client,
    base_url: &str,
    method: Method,
    path: &str,
    payload: &serde_json::Value,
) -> Result<()> {
    let url = format!("{}{}", base_url.trim_end_matches('/'), path);
    let response = client
        .request(method, url)
        .json(payload)
        .send()
        .await
        .context("sending Calvin archive write")?;
    response
        .error_for_status()
        .context("Calvin archive write returned an error status")?;
    Ok(())
}

async fn mark_calvin_write_confirmed(pool: &SqlitePool, queue_id: &str) -> Result<()> {
    let now = Utc::now().to_rfc3339();
    sqlx::query(
        r#"
        UPDATE calvin_write_queue
        SET status = 'confirmed',
            updated_at = ?2,
            confirmed_at = ?2,
            last_error = NULL
        WHERE queue_id = ?1
        "#,
    )
    .bind(queue_id)
    .bind(now)
    .execute(pool)
    .await
    .context("marking Calvin write confirmed")?;
    Ok(())
}

async fn mark_calvin_write_retry(
    pool: &SqlitePool,
    queue_id: &str,
    attempts: i64,
    error: String,
) -> Result<()> {
    let next_attempts = attempts.saturating_add(1);
    let delay_seconds = retry_delay_seconds(next_attempts);
    let now = Utc::now();
    let next_attempt_at = (now + ChronoDuration::seconds(delay_seconds)).to_rfc3339();
    sqlx::query(
        r#"
        UPDATE calvin_write_queue
        SET status = 'retry_pending',
            attempts = ?2,
            next_attempt_at = ?3,
            last_error = ?4,
            updated_at = ?5
        WHERE queue_id = ?1
        "#,
    )
    .bind(queue_id)
    .bind(next_attempts)
    .bind(next_attempt_at)
    .bind(error)
    .bind(now.to_rfc3339())
    .execute(pool)
    .await
    .context("marking Calvin write retry_pending")?;
    Ok(())
}

fn retry_delay_seconds(attempts: i64) -> i64 {
    let capped = attempts.clamp(1, 6);
    2_i64.pow(capped as u32)
}

pub async fn load_write_queue_stats(pool: &SqlitePool) -> Result<CalvinWriteQueueStats> {
    let rows = sqlx::query(
        r#"
        SELECT status, COUNT(*) AS count
        FROM calvin_write_queue
        GROUP BY status
        "#,
    )
    .fetch_all(pool)
    .await
    .context("loading Calvin write queue stats")?;

    let mut stats = CalvinWriteQueueStats::default();
    for row in rows {
        let status: String = row.get("status");
        let count: i64 = row.get("count");
        match status.as_str() {
            "pending" => stats.pending = count,
            "retry_pending" => stats.retry_pending = count,
            "confirmed" => stats.confirmed = count,
            _ => {}
        }
    }
    if let Some(row) = sqlx::query(
        r#"
        SELECT
            MIN(created_at) AS oldest_pending_at,
            MIN(next_attempt_at) AS next_attempt_at
        FROM calvin_write_queue
        WHERE status IN ('pending', 'retry_pending')
        "#,
    )
    .fetch_optional(pool)
    .await
    .context("loading Calvin write queue pending age")?
    {
        stats.oldest_pending_at = row.get::<Option<String>, _>("oldest_pending_at");
        stats.next_attempt_at = row.get::<Option<String>, _>("next_attempt_at");
    }
    if let Some(row) = sqlx::query(
        r#"
        SELECT last_error
        FROM calvin_write_queue
        WHERE last_error IS NOT NULL
          AND last_error != ''
        ORDER BY updated_at DESC
        LIMIT 1
        "#,
    )
    .fetch_optional(pool)
    .await
    .context("loading Calvin write queue last error")?
    {
        stats.last_error = row.get::<Option<String>, _>("last_error");
    }
    Ok(stats)
}

/// Try to create a CalvinClient; returns None only if disabled or local init fails.
///
/// Harmony reachability is intentionally not a precondition once the durable
/// queue is available: writes must still be accepted locally during an outage
/// and drained when harmony recovers.
pub async fn try_connect(config: &CalvinConfig, queue_pool: SqlitePool) -> Option<CalvinClient> {
    if !config.enabled {
        return None;
    }
    match CalvinClient::new_with_pool(config, queue_pool) {
        Ok(client) => {
            if client.health_check().await {
                tracing::debug!("Calvin Archive harmony reachable at {}", config.harmony_url);
            } else {
                tracing::warn!(
                    "Calvin Archive enabled but harmony is not responding at {}; archive writes will queue locally",
                    config.harmony_url
                );
            }
            client.spawn_write_queue_processor();
            Some(client)
        }
        Err(e) => {
            tracing::warn!("Calvin Archive disabled — client init failed: {e}");
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{
        drain_calvin_write_queue_once, try_connect, BeliefRevision, CalvinClient,
        CausalLinkPayload, PearlLevel, RevisionType, TelemetryEvent,
    };
    use crate::setup::CalvinConfig;
    use axum::{extract::State, http::StatusCode, routing::post, Router};
    use sqlx::{sqlite::SqlitePoolOptions, Row, SqlitePool};
    use tokio::sync::mpsc;

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

    async fn queue_pool() -> SqlitePool {
        let pool = SqlitePoolOptions::new()
            .max_connections(1)
            .connect("sqlite::memory:")
            .await
            .expect("sqlite memory pool");
        sqlx::query(
            r#"
            CREATE TABLE calvin_write_queue (
                queue_id TEXT PRIMARY KEY,
                run_id TEXT,
                operation TEXT NOT NULL,
                method TEXT NOT NULL,
                path TEXT NOT NULL,
                payload_json TEXT NOT NULL,
                status TEXT NOT NULL,
                attempts INTEGER NOT NULL DEFAULT 0,
                next_attempt_at TEXT NOT NULL,
                last_error TEXT,
                created_at TEXT NOT NULL,
                updated_at TEXT NOT NULL,
                confirmed_at TEXT
            )
            "#,
        )
        .execute(&pool)
        .await
        .expect("create queue table");
        pool
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

    #[tokio::test]
    async fn write_methods_enqueue_instead_of_sending_when_pool_is_available() {
        let pool = queue_pool().await;
        let client = CalvinClient::new_with_pool(&CalvinConfig::default(), pool.clone())
            .expect("client with queue");
        let event = TelemetryEvent {
            agent_id: "coobie".to_string(),
            run_id: "run-1".to_string(),
            phase: Some("validation".to_string()),
            action_type: "check".to_string(),
            provider: None,
            model: None,
            outcome: "ok".to_string(),
            latency_ms: Some(12),
            tokens_in: None,
            tokens_out: None,
            drift_score: None,
            lab_ness_score: None,
        };

        client.write_event(&event).await.expect("enqueue telemetry");

        let row = sqlx::query(
            "SELECT run_id, operation, method, path, status, attempts, payload_json FROM calvin_write_queue",
        )
        .fetch_one(&pool)
        .await
        .expect("queued row");

        assert_eq!(row.get::<String, _>("run_id"), "run-1");
        assert_eq!(row.get::<String, _>("operation"), "write_event");
        assert_eq!(row.get::<String, _>("method"), "POST");
        assert_eq!(row.get::<String, _>("path"), "/telemetry");
        assert_eq!(row.get::<String, _>("status"), "pending");
        assert_eq!(row.get::<i64, _>("attempts"), 0);
        let payload: serde_json::Value =
            serde_json::from_str(&row.get::<String, _>("payload_json")).expect("payload json");
        assert_eq!(payload["agent_id"], "coobie");
    }

    #[tokio::test]
    async fn drain_confirms_successful_queued_write() {
        async fn capture(
            State(tx): State<mpsc::UnboundedSender<String>>,
            body: String,
        ) -> StatusCode {
            tx.send(body).expect("capture body");
            StatusCode::CREATED
        }

        let pool = queue_pool().await;
        let (tx, mut rx) = mpsc::unbounded_channel();
        let app = Router::new()
            .route("/telemetry", post(capture))
            .with_state(tx);
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
            .await
            .expect("bind listener");
        let addr = listener.local_addr().expect("local addr");
        tokio::spawn(async move {
            axum::serve(listener, app).await.expect("serve mock Calvin");
        });

        let config = CalvinConfig {
            harmony_url: format!("http://{addr}"),
            ..CalvinConfig::default()
        };
        let client = CalvinClient::new_with_pool(&config, pool.clone()).expect("client");
        let event = TelemetryEvent {
            agent_id: "coobie".to_string(),
            run_id: "run-1".to_string(),
            phase: None,
            action_type: "write".to_string(),
            provider: None,
            model: None,
            outcome: "ok".to_string(),
            latency_ms: None,
            tokens_in: None,
            tokens_out: None,
            drift_score: None,
            lab_ness_score: None,
        };
        client.write_event(&event).await.expect("enqueue event");

        let processed =
            drain_calvin_write_queue_once(&pool, &client.write_client, &client.base_url)
                .await
                .expect("drain");

        assert_eq!(processed, 1);
        let body = rx.recv().await.expect("captured request");
        assert!(body.contains("\"agent_id\":\"coobie\""));
        let row = sqlx::query("SELECT status, confirmed_at FROM calvin_write_queue")
            .fetch_one(&pool)
            .await
            .expect("confirmed row");
        assert_eq!(row.get::<String, _>("status"), "confirmed");
        assert!(row.get::<Option<String>, _>("confirmed_at").is_some());
    }

    #[tokio::test]
    async fn enabled_archive_still_returns_client_when_harmony_is_down() {
        let pool = queue_pool().await;
        let config = CalvinConfig {
            enabled: true,
            harmony_url: "http://127.0.0.1:9".to_string(),
            health_timeout_ms: 25,
            write_timeout_ms: 25,
            read_timeout_ms: 25,
        };

        let client = try_connect(&config, pool.clone())
            .await
            .expect("queue-backed client should exist during harmony outage");
        let event = TelemetryEvent {
            agent_id: "coobie".to_string(),
            run_id: "run-outage".to_string(),
            phase: None,
            action_type: "write".to_string(),
            provider: None,
            model: None,
            outcome: "queued".to_string(),
            latency_ms: None,
            tokens_in: None,
            tokens_out: None,
            drift_score: None,
            lab_ness_score: None,
        };

        client
            .write_event(&event)
            .await
            .expect("outage write should enqueue locally");

        let row = sqlx::query("SELECT run_id, status FROM calvin_write_queue")
            .fetch_one(&pool)
            .await
            .expect("queued outage write");
        assert_eq!(row.get::<String, _>("run_id"), "run-outage");
        assert_eq!(row.get::<String, _>("status"), "pending");
    }

    #[tokio::test]
    async fn queue_stats_include_retry_context() {
        let pool = queue_pool().await;
        sqlx::query(
            r#"
            INSERT INTO calvin_write_queue
                (queue_id, run_id, operation, method, path, payload_json, status,
                 attempts, next_attempt_at, last_error, created_at, updated_at)
            VALUES
                ('q-pending', 'run-1', 'write_event', 'POST', '/telemetry', '{}',
                 'pending', 0, '2026-05-01T10:05:00Z', NULL,
                 '2026-05-01T10:00:00Z', '2026-05-01T10:00:00Z'),
                ('q-retry', 'run-2', 'write_event', 'POST', '/telemetry', '{}',
                 'retry_pending', 2, '2026-05-01T10:04:00Z', 'connection refused',
                 '2026-05-01T10:01:00Z', '2026-05-01T10:02:00Z'),
                ('q-confirmed', 'run-3', 'write_event', 'POST', '/telemetry', '{}',
                 'confirmed', 1, '2026-05-01T10:03:00Z', NULL,
                 '2026-05-01T09:59:00Z', '2026-05-01T10:03:00Z')
            "#,
        )
        .execute(&pool)
        .await
        .expect("insert stats rows");

        let stats = super::load_write_queue_stats(&pool)
            .await
            .expect("queue stats");

        assert_eq!(stats.pending, 1);
        assert_eq!(stats.retry_pending, 1);
        assert_eq!(stats.confirmed, 1);
        assert_eq!(stats.pending_confirmation_count(), 2);
        assert_eq!(
            stats.oldest_pending_at.as_deref(),
            Some("2026-05-01T10:00:00Z")
        );
        assert_eq!(
            stats.next_attempt_at.as_deref(),
            Some("2026-05-01T10:04:00Z")
        );
        assert_eq!(stats.last_error.as_deref(), Some("connection refused"));
    }

    #[test]
    fn causal_link_payload_downgrades_unwarranted_pearl_level() {
        let payload = CausalLinkPayload {
            cause_episode_id: "cause-1".to_string(),
            effect_episode_id: "effect-1".to_string(),
            pearl_level: PearlLevel::Counterfactual,
            confidence: 0.7,
            held_fixed: vec!["api_version".to_string()],
            estimated_effect_delta: Some(0.4),
            actual_trace_id: None,
            hypothetical_intervention: None,
            warrant_gap: false,
            epistemic_warrant: None,
        }
        .normalized()
        .expect("normalized payload");

        assert_eq!(payload.pearl_level, PearlLevel::Interventional);
        assert_eq!(payload.epistemic_warrant, Some(PearlLevel::Interventional));
        assert!(payload.warrant_gap);
    }

    #[tokio::test]
    async fn structural_causal_link_enqueue_records_warrant_gap() {
        let pool = queue_pool().await;
        let client = CalvinClient::new_with_pool(&CalvinConfig::default(), pool.clone())
            .expect("client with queue");
        let payload = CausalLinkPayload {
            cause_episode_id: "cause-1".to_string(),
            effect_episode_id: "effect-1".to_string(),
            pearl_level: PearlLevel::Interventional,
            confidence: 0.6,
            held_fixed: Vec::new(),
            estimated_effect_delta: None,
            actual_trace_id: None,
            hypothetical_intervention: None,
            warrant_gap: false,
            epistemic_warrant: None,
        };

        client
            .record_structural_causal_link("run-1", payload)
            .await
            .expect("enqueue causal link");

        let row = sqlx::query("SELECT payload_json FROM calvin_write_queue")
            .fetch_one(&pool)
            .await
            .expect("queued causal link");
        let payload: serde_json::Value =
            serde_json::from_str(&row.get::<String, _>("payload_json")).expect("payload json");
        assert_eq!(payload["pearl_level"], "Associational");
        assert_eq!(payload["epistemic_warrant"], "Associational");
        assert_eq!(payload["warrant_gap"], true);
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
