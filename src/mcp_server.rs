use anyhow::{bail, Context, Result};
use axum::{
    extract::State,
    http::StatusCode,
    response::{
        sse::{Event, KeepAlive, Sse},
        IntoResponse, Response,
    },
    routing::{get, post},
    Json, Router,
};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::{convert::Infallible, net::SocketAddr, sync::Arc, time::Instant};
use tokio_stream::{wrappers::BroadcastStream, StreamExt as _};
use tracing::info;

use crate::{
    cli::McpServeArgs,
    orchestrator::{AppContext, RunRequest},
    reporting,
};

#[derive(Clone)]
struct McpState {
    app: AppContext,
    started_at: Instant,
}

#[derive(Debug, Serialize)]
struct McpHealthResponse {
    status: &'static str,
    transport: &'static str,
    uptime_secs: u64,
    version: &'static str,
}

#[derive(Debug, Deserialize)]
struct RpcEnvelope {
    #[serde(default)]
    id: Option<Value>,
    #[serde(default)]
    method: String,
    #[serde(default)]
    params: Value,
}

pub async fn handle_mcp_serve(app: AppContext, args: McpServeArgs) -> Result<()> {
    let configured = app
        .paths
        .setup
        .mcp
        .as_ref()
        .and_then(|mcp| mcp.self_server.as_ref());

    let transport = args
        .transport
        .clone()
        .or_else(|| configured.map(|cfg| cfg.transport.clone()))
        .unwrap_or_else(|| "sse".to_string());

    match transport.as_str() {
        "sse" => {
            let host = args
                .host
                .clone()
                .or_else(|| configured.and_then(|cfg| cfg.host.clone()))
                .unwrap_or_else(|| "127.0.0.1".to_string());
            let port = args
                .port
                .or_else(|| configured.and_then(|cfg| cfg.port))
                .unwrap_or(3001);
            start_sse_server(app, &host, port).await
        }
        "stdio" => bail!("stdio transport is not implemented yet; use --transport sse for now"),
        other => bail!("unsupported MCP transport: {other}"),
    }
}

async fn start_sse_server(app: AppContext, host: &str, port: u16) -> Result<()> {
    let addr: SocketAddr = format!("{host}:{port}")
        .parse()
        .with_context(|| format!("invalid MCP self-server address: {host}:{port}"))?;
    let state = Arc::new(McpState {
        app,
        started_at: Instant::now(),
    });
    let router = Router::new()
        .route("/health", get(get_health))
        .route("/sse", get(get_sse))
        .route("/rpc", post(post_rpc))
        .with_state(state);

    info!(%addr, "starting Harkonnen MCP self-server");
    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, router).await?;
    Ok(())
}

async fn get_health(State(state): State<Arc<McpState>>) -> impl IntoResponse {
    Json(McpHealthResponse {
        status: "ok",
        transport: "sse",
        uptime_secs: state.started_at.elapsed().as_secs(),
        version: env!("CARGO_PKG_VERSION"),
    })
}

async fn get_sse(State(state): State<Arc<McpState>>) -> impl IntoResponse {
    let rx = state.app.event_tx.subscribe();
    let stream = BroadcastStream::new(rx).filter_map(|item| {
        match item {
            Ok(event) => {
                let event = Event::default()
                    .event("live-event")
                    .json_data(&event)
                    .unwrap_or_else(|_| Event::default().event("live-event").data("{}"));
                Some(Ok::<Event, Infallible>(event))
            }
            Err(_) => None,
        }
    });

    Sse::new(stream).keep_alive(
        KeepAlive::new()
            .interval(std::time::Duration::from_secs(15))
            .text("keepalive"),
    )
}

async fn post_rpc(
    State(state): State<Arc<McpState>>,
    Json(body): Json<Value>,
) -> Response {
    if body.is_array() {
        return rpc_error(None, -32600, "batch requests are not supported").into_response();
    }

    let envelope: RpcEnvelope = match serde_json::from_value(body) {
        Ok(value) => value,
        Err(error) => {
            return rpc_error(None, -32600, &format!("invalid request: {error}")).into_response()
        }
    };

    match handle_rpc(&state, &envelope).await {
        Ok(Some(result)) => Json(result).into_response(),
        Ok(None) => StatusCode::NO_CONTENT.into_response(),
        Err((code, message)) => rpc_error(envelope.id.clone(), code, &message).into_response(),
    }
}

async fn handle_rpc(state: &McpState, envelope: &RpcEnvelope) -> std::result::Result<Option<Value>, (i64, String)> {
    let id = envelope.id.clone();
    let method = envelope.method.trim();
    if method.is_empty() {
        return Err((-32600, "missing method".to_string()));
    }

    let result = match method {
        "initialize" => json!({
            "protocolVersion": "2024-11-05",
            "capabilities": {
                "tools": { "listChanged": false },
                "resources": { "listChanged": false, "subscribe": false },
                "prompts": { "listChanged": false }
            },
            "serverInfo": {
                "name": "harkonnen-labs",
                "version": env!("CARGO_PKG_VERSION")
            },
            "instructions": "Harkonnen exposes factory runs, reports, decision logs, and commissioning actions through a minimal MCP-compatible self-server."
        }),
        "notifications/initialized" => return Ok(None),
        "ping" => json!({ "ok": true }),
        "tools/list" => json!({ "tools": tool_descriptors() }),
        "resources/list" => json!({ "resources": resource_descriptors() }),
        "prompts/list" => json!({ "prompts": prompt_descriptors() }),
        "resources/read" => read_resource(state, &envelope.params).await?,
        "prompts/get" => get_prompt(&envelope.params)?,
        "tools/call" => call_tool(state, &envelope.params).await?,
        _ => return Err((-32601, format!("method not found: {method}"))),
    };

    Ok(Some(json!({
        "jsonrpc": "2.0",
        "id": id,
        "result": result
    })))
}

async fn call_tool(state: &McpState, params: &Value) -> std::result::Result<Value, (i64, String)> {
    let name = params
        .get("name")
        .and_then(Value::as_str)
        .ok_or_else(|| (-32602, "tools/call requires params.name".to_string()))?;
    let arguments = params.get("arguments").cloned().unwrap_or_else(|| json!({}));

    let result = match name {
        "list_runs" => {
            let limit = arguments
                .get("limit")
                .and_then(Value::as_i64)
                .unwrap_or(20)
                .clamp(1, 100);
            let runs = state
                .app
                .list_runs(limit)
                .await
                .map_err(internal_error)?;
            json!(runs)
        }
        "get_run" => {
            let run_id = required_string(&arguments, "run_id")?;
            let run = state
                .app
                .get_run(&run_id)
                .await
                .map_err(internal_error)?
                .ok_or_else(|| (-32004, format!("run not found: {run_id}")))?;
            json!(run)
        }
        "get_run_report" => {
            let run_id = required_string(&arguments, "run_id")?;
            let report = reporting::build_report(&state.app, &run_id)
                .await
                .map_err(internal_error)?;
            return Ok(text_tool_result(&report));
        }
        "list_run_decisions" => {
            let run_id = required_string(&arguments, "run_id")?;
            let decisions = state
                .app
                .list_run_decisions(&run_id)
                .await
                .map_err(internal_error)?;
            json!(decisions)
        }
        "start_run" => {
            let spec = required_string(&arguments, "spec")?;
            let product = optional_string(&arguments, "product");
            let product_path = optional_string(&arguments, "product_path");
            if product.is_none() && product_path.is_none() {
                return Err((
                    -32602,
                    "start_run requires either arguments.product or arguments.product_path"
                        .to_string(),
                ));
            }
            let run_hidden_scenarios = arguments
                .get("run_hidden_scenarios")
                .and_then(Value::as_bool)
                .unwrap_or(true);
            let run = state
                .app
                .start_run(RunRequest {
                    spec_path: spec,
                    product,
                    product_path,
                    run_hidden_scenarios,
                    failure_harness: None,
                })
                .await
                .map_err(internal_error)?;
            json!({
                "run_id": run.run_id,
                "status": run.status,
                "spec_id": run.spec_id,
                "product": run.product
            })
        }
        _ => return Err((-32601, format!("unknown tool: {name}"))),
    };

    Ok(text_tool_result_pretty(&result))
}

async fn read_resource(state: &McpState, params: &Value) -> std::result::Result<Value, (i64, String)> {
    let uri = required_string(params, "uri")?;
    let (mime_type, payload) = if uri == "harkonnen://runs" {
        let runs = state.app.list_runs(20).await.map_err(internal_error)?;
        ("application/json", serde_json::to_string_pretty(&runs).map_err(internal_error)?)
    } else if let Some(run_id) = uri.strip_prefix("harkonnen://runs/") {
        let run = state
            .app
            .get_run(run_id)
            .await
            .map_err(internal_error)?
            .ok_or_else(|| (-32004, format!("run not found: {run_id}")))?;
        (
            "application/json",
            serde_json::to_string_pretty(&run).map_err(internal_error)?,
        )
    } else if let Some(run_id) = uri.strip_prefix("harkonnen://reports/") {
        let report = reporting::build_report(&state.app, run_id)
            .await
            .map_err(internal_error)?;
        ("text/plain", report)
    } else {
        return Err((-32602, format!("unknown resource URI: {uri}")));
    };

    Ok(json!({
        "contents": [
            {
                "uri": uri,
                "mimeType": mime_type,
                "text": payload
            }
        ]
    }))
}

fn get_prompt(params: &Value) -> std::result::Result<Value, (i64, String)> {
    let name = required_string(params, "name")?;
    let arguments = params.get("arguments").cloned().unwrap_or_else(|| json!({}));
    let text = match name.as_str() {
        "briefing_for_spec" => {
            let spec_id = required_string(&arguments, "spec_id")?;
            format!(
                "Build a concise Harkonnen briefing for spec `{spec_id}`. Include likely risks, guardrails, and the next recommended operator action."
            )
        }
        "diagnose_run" => {
            let run_id = required_string(&arguments, "run_id")?;
            format!(
                "Diagnose Harkonnen run `{run_id}`. Summarize status, likely failure causes, operator-visible risks, and the most useful next debugging step."
            )
        }
        _ => return Err((-32601, format!("unknown prompt: {name}"))),
    };

    Ok(json!({
        "description": format!("Prompt template `{name}`"),
        "messages": [
            {
                "role": "user",
                "content": {
                    "type": "text",
                    "text": text
                }
            }
        ]
    }))
}

fn required_string(params: &Value, key: &str) -> std::result::Result<String, (i64, String)> {
    params
        .get(key)
        .and_then(Value::as_str)
        .map(|value| value.to_string())
        .filter(|value| !value.trim().is_empty())
        .ok_or_else(|| (-32602, format!("missing required argument: {key}")))
}

fn optional_string(params: &Value, key: &str) -> Option<String> {
    params
        .get(key)
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(|value| value.to_string())
}

fn internal_error(error: impl ToString) -> (i64, String) {
    (-32000, error.to_string())
}

fn rpc_error(id: Option<Value>, code: i64, message: &str) -> Json<Value> {
    Json(json!({
        "jsonrpc": "2.0",
        "id": id,
        "error": {
            "code": code,
            "message": message
        }
    }))
}

fn text_tool_result(text: &str) -> Value {
    json!({
        "content": [
            {
                "type": "text",
                "text": text
            }
        ]
    })
}

fn text_tool_result_pretty(value: &Value) -> Value {
    let rendered = serde_json::to_string_pretty(value)
        .unwrap_or_else(|_| value.to_string());
    text_tool_result(&rendered)
}

fn tool_descriptors() -> Vec<Value> {
    vec![
        json!({
            "name": "list_runs",
            "description": "List recent Harkonnen runs.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "limit": { "type": "integer", "minimum": 1, "maximum": 100 }
                }
            }
        }),
        json!({
            "name": "get_run",
            "description": "Fetch a specific run record.",
            "inputSchema": {
                "type": "object",
                "required": ["run_id"],
                "properties": {
                    "run_id": { "type": "string" }
                }
            }
        }),
        json!({
            "name": "get_run_report",
            "description": "Render the full text run report for a specific run.",
            "inputSchema": {
                "type": "object",
                "required": ["run_id"],
                "properties": {
                    "run_id": { "type": "string" }
                }
            }
        }),
        json!({
            "name": "list_run_decisions",
            "description": "List decision log records for a run.",
            "inputSchema": {
                "type": "object",
                "required": ["run_id"],
                "properties": {
                    "run_id": { "type": "string" }
                }
            }
        }),
        json!({
            "name": "start_run",
            "description": "Start a new Harkonnen run from a spec path and product target.",
            "inputSchema": {
                "type": "object",
                "required": ["spec"],
                "properties": {
                    "spec": { "type": "string" },
                    "product": { "type": "string" },
                    "product_path": { "type": "string" },
                    "run_hidden_scenarios": { "type": "boolean" }
                }
            }
        }),
    ]
}

fn resource_descriptors() -> Vec<Value> {
    vec![
        json!({
            "uri": "harkonnen://runs",
            "name": "Recent Runs",
            "description": "Recent Harkonnen run records.",
            "mimeType": "application/json"
        }),
        json!({
            "uriTemplate": "harkonnen://runs/{run_id}",
            "name": "Run Detail",
            "description": "A single Harkonnen run record.",
            "mimeType": "application/json"
        }),
        json!({
            "uriTemplate": "harkonnen://reports/{run_id}",
            "name": "Run Report",
            "description": "Rendered text report for a Harkonnen run.",
            "mimeType": "text/plain"
        }),
    ]
}

fn prompt_descriptors() -> Vec<Value> {
    vec![
        json!({
            "name": "briefing_for_spec",
            "description": "Prompt template for building a Coobie-style briefing for a spec."
        }),
        json!({
            "name": "diagnose_run",
            "description": "Prompt template for diagnosing a completed or failed run."
        }),
    ]
}
