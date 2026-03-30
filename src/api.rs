use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
    routing::get,
    Json, Router,
};
use std::net::SocketAddr;
use tower_http::cors::{Any, CorsLayer};
use crate::orchestrator::AppContext;
use tracing::info;

pub async fn start_api_server(app: AppContext, port: u16) -> anyhow::Result<()> {
    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);

    let router = Router::new()
        .route("/api/runs", get(list_runs))
        .route("/api/runs/:id", get(get_run))
        .route("/api/runs/:id/events", get(get_run_events))
        .layer(cors)
        .with_state(app);

    let addr = SocketAddr::from(([127, 0, 0, 1], port));
    info!("API server listening on http://{}", addr);
    
    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, router).await?;

    Ok(())
}

async fn list_runs(
    State(app): State<AppContext>,
) -> impl IntoResponse {
    match app.list_runs(50).await {
        Ok(runs) => (StatusCode::OK, Json(runs)).into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    }
}

async fn get_run(
    Path(id): Path<String>,
    State(app): State<AppContext>,
) -> impl IntoResponse {
    match app.get_run(&id).await {
        Ok(Some(run)) => (StatusCode::OK, Json(run)).into_response(),
        Ok(None) => (StatusCode::NOT_FOUND, "Run not found").into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    }
}

async fn get_run_events(
    Path(id): Path<String>,
    State(app): State<AppContext>,
) -> impl IntoResponse {
    match app.list_run_events(&id).await {
        Ok(events) => (StatusCode::OK, Json(events)).into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    }
}
