use anyhow::{Context, Result};
use sqlx::PgPool;

use crate::archive::ArchiveStore;

#[derive(Debug, Clone, serde::Serialize)]
pub(crate) struct MetricsSnapshot {
    pub agent_id: String,
    pub d_star: f64,
    pub ssa: f64,
    pub stress: f64,
    pub hysteresis: Option<f64>,
    pub computed_at: chrono::DateTime<chrono::Utc>,
}

/// D* = α / γ  (drift bound; stability requires D* < 1.0)
/// α = avg drift_score over window, γ = success rate (recovery rate proxy)
pub(crate) async fn compute_d_star(pool: &PgPool, agent_id: &str, window_min: i32) -> Result<f64> {
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
    .fetch_optional(pool)
    .await
    .context("compute_d_star")?;

    match row {
        Some((Some(alpha), Some(gamma))) if gamma > 0.0 => Ok(alpha / gamma),
        _ => Ok(0.0),
    }
}

/// SSA = mean lab_ness_score across all behavioral signatures for the agent.
/// Approximates cross-domain action-pattern consistency vs Labrador persona goals.
pub(crate) async fn compute_ssa(archive: &ArchiveStore, agent_id: &str) -> Result<f64> {
    // Query TypeDB for behavioral_signature lab_ness_score values
    // Approximation: average lab_ness_score from recent telemetry for this agent
    let _ = archive; // TypeDB query path used in get_kernel_traits; reuse pool below
    let _ = agent_id;
    // Phase 8-C will add the full TypeDB query; for now return a default
    Ok(1.0)
}

/// S(T) = Σ w_i * drift_score_i * exp(-λ * (T - t_i))
/// λ = 0.1 (privileges last ~10 minutes); computed over last 60 minutes
pub(crate) async fn compute_stress(pool: &PgPool, agent_id: &str) -> Result<f64> {
    let rows: Vec<(f64, f64)> = sqlx::query_as(
        r#"SELECT
            drift_score,
            EXTRACT(EPOCH FROM (NOW() - time)) / 60.0 AS age_minutes
           FROM agent_telemetry
           WHERE agent_id = $1
             AND drift_score IS NOT NULL
             AND time > NOW() - INTERVAL '60 minutes'
           ORDER BY time DESC"#,
    )
    .bind(agent_id)
    .fetch_all(pool)
    .await
    .context("compute_stress")?;

    let lambda = 0.1_f64;
    let stress: f64 = rows
        .iter()
        .map(|(drift, age_min)| drift * (-lambda * age_min).exp())
        .sum();

    Ok(stress.min(1.0))
}

/// H = Δ_post_rollback / Δ_at_peak
/// Compares continuity_index before and after a rollback event.
/// Requires continuity_snapshot pairs in TypeDB. Returns None if no rollback found.
pub(crate) async fn compute_hysteresis(
    _archive: &ArchiveStore,
    _run_id: &str,
) -> Result<Option<f64>> {
    // Phase 8-C full implementation; no rollback data in Phase 6
    Ok(None)
}

pub(crate) async fn full_snapshot(
    pool: &PgPool,
    archive: &ArchiveStore,
    agent_id: &str,
) -> Result<MetricsSnapshot> {
    let (d_star, ssa, stress, hysteresis) = tokio::try_join!(
        compute_d_star(pool, agent_id, 30),
        compute_ssa(archive, agent_id),
        compute_stress(pool, agent_id),
        async { compute_hysteresis(archive, agent_id).await },
    )?;

    Ok(MetricsSnapshot {
        agent_id: agent_id.to_string(),
        d_star,
        ssa,
        stress,
        hysteresis,
        computed_at: chrono::Utc::now(),
    })
}
