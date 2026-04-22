mod api;
mod archive;
mod metrics;
mod streaming;
mod telemetry;

use anyhow::Result;
use std::sync::Arc;
use tokio::net::TcpListener;
use tracing::info;
use tracing_subscriber::EnvFilter;

pub(crate) struct CalvinState {
    pub(crate) archive: archive::ArchiveStore,
    pub(crate) telemetry: Option<telemetry::TimescaleWriter>,
}

#[tokio::main]
async fn main() -> Result<()> {
    let _ = dotenvy();
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env())
        .init();

    let typedb_url = std::env::var("TYPEDB_URL").unwrap_or_else(|_| "localhost:1729".into());
    let timescale_url = std::env::var("TIMESCALE_URL").unwrap_or_else(|_| {
        "postgres://harkonnen:harkonnen@localhost:5432/harkonnen_telemetry".into()
    });
    let materialize_url = std::env::var("MATERIALIZE_URL")
        .unwrap_or_else(|_| "postgres://materialize@localhost:6875/materialize".into());
    let enable_telemetry = env_flag("CALVIN_ENABLE_TELEMETRY", false);
    let enable_streaming = env_flag("CALVIN_ENABLE_STREAMING", false);
    let port: u16 = std::env::var("CALVIN_PORT")
        .unwrap_or_else(|_| "7171".into())
        .parse()
        .unwrap_or(7171);
    let db_name = std::env::var("CALVIN_DB").unwrap_or_else(|_| "harkonnen".into());

    info!("Calvin Archive starting — TypeDB: {typedb_url} TimescaleDB: {timescale_url}");

    let archive = archive::ArchiveStore::connect(&typedb_url, &db_name).await?;
    archive.deploy_schema().await?;
    archive.seed_coobie_kernel().await?;
    info!("TypeDB schema deployed and Coobie kernel seeded");

    let telemetry = if enable_telemetry {
        let telemetry = telemetry::TimescaleWriter::connect(&timescale_url).await?;
        telemetry.deploy_schema().await?;
        info!("TimescaleDB schema ready");
        Some(telemetry)
    } else {
        info!("Timescale telemetry disabled for minimal archive-first boot");
        None
    };

    let state = Arc::new(CalvinState { archive, telemetry });

    if enable_streaming {
        if state.telemetry.is_some() {
            let mat_client = streaming::MaterializeClient::connect(&materialize_url).await?;
            mat_client.deploy_views().await?;
            info!("Materialize views ready");

            let (alert_tx, alert_rx) = tokio::sync::mpsc::channel(64);
            let mat_clone = mat_client;
            let alert_tx_clone = alert_tx;
            tokio::spawn(async move {
                if let Err(e) = mat_clone.subscribe_drift_alerts(alert_tx_clone).await {
                    tracing::warn!("Materialize SUBSCRIBE ended: {e}");
                }
            });

            let state_clone = Arc::clone(&state);
            tokio::spawn(async move {
                streaming::MetaGovernor::new(state_clone, alert_rx)
                    .run()
                    .await;
            });
        } else {
            tracing::warn!(
                "CALVIN_ENABLE_STREAMING=true but telemetry is disabled; skipping Materialize and Meta-Governor"
            );
        }
    } else {
        info!("Materialize streaming disabled for minimal archive-first boot");
    }

    let app = api::router(state);
    let listener = TcpListener::bind(format!("0.0.0.0:{port}")).await?;
    info!("Calvin Archive listening on port {port}");
    axum::serve(listener, app).await?;
    Ok(())
}

fn dotenvy() -> Result<()> {
    let _ = std::fs::read_to_string(".env").ok().map(|content| {
        for line in content.lines() {
            let line = line.trim();
            if line.is_empty() || line.starts_with('#') {
                return;
            }
            if let Some((k, v)) = line.split_once('=') {
                std::env::set_var(k.trim(), v.trim().trim_matches('"'));
            }
        }
    });
    Ok(())
}

fn env_flag(name: &str, default: bool) -> bool {
    std::env::var(name)
        .ok()
        .map(|value| {
            matches!(
                value.trim().to_ascii_lowercase().as_str(),
                "1" | "true" | "yes" | "on"
            )
        })
        .unwrap_or(default)
}
