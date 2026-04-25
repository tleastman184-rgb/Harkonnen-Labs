use anyhow::{Context, Result};
use sqlx::PgPool;
use std::sync::Arc;
use tokio::sync::mpsc;

const VIEWS_SQL: &str = include_str!("../../factory/calvin_archive/materialize/views.sql");

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
pub(crate) struct DriftAlert {
    pub agent_id: String,
    pub failure_count: i64,
    pub avg_drift: f64,
    pub d_star: f64,
    pub alpha: f64,
    pub gamma: f64,
    pub window_minutes: i32,
    pub alert_at: chrono::DateTime<chrono::Utc>,
}

pub(crate) struct MaterializeClient {
    pool: PgPool,
}

impl MaterializeClient {
    pub(crate) async fn connect(url: &str) -> Result<Self> {
        let pool = PgPool::connect(url)
            .await
            .with_context(|| format!("connecting to Materialize at {url}"))?;
        Ok(Self { pool })
    }

    pub(crate) async fn deploy_views(&self) -> Result<()> {
        // Run view definitions — Materialize uses CREATE OR REPLACE so these are idempotent
        // Split on semicolons to execute each statement individually
        for stmt in VIEWS_SQL.split(';') {
            let stmt = stmt.trim();
            if stmt.is_empty() || stmt.starts_with("--") {
                continue;
            }
            if let Err(e) = sqlx::raw_sql(&format!("{stmt};")).execute(&self.pool).await {
                tracing::warn!("Materialize view deploy (non-fatal): {e}");
            }
        }
        Ok(())
    }

    pub(crate) async fn subscribe_drift_alerts(
        &self,
        alert_tx: mpsc::Sender<DriftAlert>,
    ) -> Result<()> {
        // Poll agent_drift_alerts every 30 seconds (SUBSCRIBE not yet stable in all Materialize versions)
        loop {
            match self.poll_alerts().await {
                Ok(alerts) => {
                    for alert in alerts {
                        if alert_tx.send(alert).await.is_err() {
                            return Ok(());
                        }
                    }
                }
                Err(e) => tracing::warn!("Materialize poll error: {e}"),
            }
            tokio::time::sleep(tokio::time::Duration::from_secs(30)).await;
        }
    }

    async fn poll_alerts(&self) -> Result<Vec<DriftAlert>> {
        let rows: Vec<(String, i64, f64)> =
            sqlx::query_as("SELECT agent_id, failure_count, avg_drift FROM agent_drift_alerts")
                .fetch_all(&self.pool)
                .await
                .context("poll_alerts")?;

        Ok(rows
            .into_iter()
            .map(|(agent_id, failure_count, avg_drift)| DriftAlert {
                agent_id,
                failure_count,
                avg_drift,
                d_star: if avg_drift > 0.0 { avg_drift } else { 0.0 },
                alpha: avg_drift,
                gamma: 1.0 - avg_drift.min(1.0),
                window_minutes: 15,
                alert_at: chrono::Utc::now(),
            })
            .collect())
    }
}

pub(crate) struct MetaGovernor {
    state: Arc<crate::CalvinState>,
    alert_rx: mpsc::Receiver<DriftAlert>,
}

impl MetaGovernor {
    pub(crate) fn new(
        state: Arc<crate::CalvinState>,
        alert_rx: mpsc::Receiver<DriftAlert>,
    ) -> Self {
        Self { state, alert_rx }
    }

    pub(crate) async fn run(mut self) {
        while let Some(alert) = self.alert_rx.recv().await {
            if let Err(e) = self.adjudicate(alert).await {
                tracing::warn!("Meta-Governor adjudication error: {e}");
            }
        }
    }

    async fn adjudicate(&self, alert: DriftAlert) -> Result<()> {
        let traits = self
            .state
            .archive
            .get_kernel_traits(&alert.agent_id)
            .await
            .unwrap_or_default();

        let kernel_intact = !traits.is_empty();

        if alert.d_star > 1.0 {
            tracing::error!(
                agent = %alert.agent_id,
                d_star = %alert.d_star,
                "META-GOVERNOR: D* > 1.0 — Keeper escalation required"
            );
            // Write drift snapshot
            self.write_drift_snapshot(&alert).await?;
        } else if alert.d_star > 0.8 {
            tracing::warn!(
                agent = %alert.agent_id,
                d_star = %alert.d_star,
                kernel_intact = %kernel_intact,
                "META-GOVERNOR: D* > 0.8 — recovery suggestion"
            );
            self.write_drift_snapshot(&alert).await?;
        } else if alert.avg_drift > 0.7 {
            tracing::warn!(
                agent = %alert.agent_id,
                avg_drift = %alert.avg_drift,
                "META-GOVERNOR: high drift score — monitoring"
            );
        }
        Ok(())
    }

    async fn write_drift_snapshot(&self, alert: &DriftAlert) -> Result<()> {
        let Some(telemetry) = self.state.telemetry.as_ref() else {
            tracing::warn!("Meta-Governor snapshot skipped because telemetry is disabled");
            return Ok(());
        };
        let gamma = if alert.avg_drift > 0.0 {
            (1.0 - alert.avg_drift).max(0.001)
        } else {
            1.0
        };
        sqlx::query(
            r#"INSERT INTO drift_bound_snapshots (agent_id, d_star, alpha, gamma, window_min)
               VALUES ($1, $2, $3, $4, $5)"#,
        )
        .bind(&alert.agent_id)
        .bind(alert.d_star)
        .bind(alert.alpha)
        .bind(gamma)
        .bind(alert.window_minutes)
        .execute(telemetry.pool())
        .await
        .context("write_drift_snapshot")?;
        Ok(())
    }
}
