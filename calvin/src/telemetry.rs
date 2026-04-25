use anyhow::{Context, Result};
use sqlx::PgPool;

const SCHEMA_SQL: &str = include_str!("../../factory/calvin_archive/timescale/schema.sql");

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
pub(crate) struct TelemetryEvent {
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

pub(crate) struct TimescaleWriter {
    pool: PgPool,
}

impl TimescaleWriter {
    pub(crate) async fn connect(url: &str) -> Result<Self> {
        let pool = PgPool::connect(url)
            .await
            .with_context(|| format!("connecting to TimescaleDB at {url}"))?;
        Ok(Self { pool })
    }

    pub(crate) async fn deploy_schema(&self) -> Result<()> {
        // Run each statement that isn't a SELECT (DDL only).
        // TimescaleDB create_hypertable and add_retention_policy are wrapped in DO blocks
        // in the schema file so they are idempotent.
        sqlx::raw_sql(SCHEMA_SQL)
            .execute(&self.pool)
            .await
            .context("deploying TimescaleDB schema")?;
        Ok(())
    }

    pub(crate) async fn write_event(&self, evt: &TelemetryEvent) -> Result<()> {
        sqlx::query(
            r#"INSERT INTO agent_telemetry
               (agent_id, run_id, phase, action_type, provider, model,
                outcome, latency_ms, tokens_in, tokens_out, drift_score, lab_ness_score)
               VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12)"#,
        )
        .bind(&evt.agent_id)
        .bind(&evt.run_id)
        .bind(&evt.phase)
        .bind(&evt.action_type)
        .bind(&evt.provider)
        .bind(&evt.model)
        .bind(&evt.outcome)
        .bind(evt.latency_ms)
        .bind(evt.tokens_in)
        .bind(evt.tokens_out)
        .bind(evt.drift_score)
        .bind(evt.lab_ness_score)
        .execute(&self.pool)
        .await
        .context("write_event")?;
        Ok(())
    }

    pub(crate) async fn write_events_batch(&self, evts: &[TelemetryEvent]) -> Result<()> {
        let mut tx = self.pool.begin().await?;
        for evt in evts {
            sqlx::query(
                r#"INSERT INTO agent_telemetry
                   (agent_id, run_id, phase, action_type, provider, model,
                    outcome, latency_ms, tokens_in, tokens_out, drift_score, lab_ness_score)
                   VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12)"#,
            )
            .bind(&evt.agent_id)
            .bind(&evt.run_id)
            .bind(&evt.phase)
            .bind(&evt.action_type)
            .bind(&evt.provider)
            .bind(&evt.model)
            .bind(&evt.outcome)
            .bind(evt.latency_ms)
            .bind(evt.tokens_in)
            .bind(evt.tokens_out)
            .bind(evt.drift_score)
            .bind(evt.lab_ness_score)
            .execute(&mut *tx)
            .await
            .context("write_events_batch")?;
        }
        tx.commit().await?;
        Ok(())
    }

    pub(crate) async fn query_d_star(&self, agent_id: &str, window_min: i32) -> Result<f64> {
        let row: Option<(Option<f64>, Option<f64>)> = sqlx::query_as(
            r#"SELECT
                AVG(drift_score) AS alpha,
                AVG(CASE WHEN outcome = 'success' THEN 1.0 ELSE 0.0 END) AS gamma
               FROM agent_telemetry
               WHERE agent_id = $1
                 AND time > NOW() - ($2 || ' minutes')::INTERVAL"#,
        )
        .bind(agent_id)
        .bind(window_min)
        .fetch_optional(&self.pool)
        .await
        .context("query_d_star")?;

        match row {
            Some((Some(alpha), Some(gamma))) if gamma > 0.0 => Ok(alpha / gamma),
            _ => Ok(0.0),
        }
    }

    pub(crate) fn pool(&self) -> &PgPool {
        &self.pool
    }
}
