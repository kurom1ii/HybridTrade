use std::{convert::Infallible, str::FromStr, sync::Arc, time::Duration};

use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::{
        sse::{Event, KeepAlive, Sse},
        IntoResponse, Response,
    },
    routing::{get, patch, post},
    Json, Router,
};
use futures_util::{Stream, StreamExt};
use serde_json::json;
use tokio::sync::{broadcast, mpsc};
use tokio_stream::wrappers::UnboundedReceiverStream;

use crate::{
    agents::{
        AgentChatOptions, AgentRole, ChatStreamEvent, DebugAgentChatRequest,
        DebugAgentView, ProviderStatusView,
    },
    db, AppState,
};

use tracing::info;

pub fn router(state: Arc<AppState>) -> Router {
    Router::new()
        .route("/health", get(health))
        .route("/api/dashboard", get(dashboard))
        .route("/api/agents/status", get(agent_status))
        .route("/api/schedules", get(list_schedules).post(create_schedule))
        .route("/api/schedules/stream", get(schedule_stream))
        .route("/api/schedules/:id", patch(update_schedule).delete(delete_schedule))
        .route("/api/instruments", get(list_instruments))
        .route("/api/instruments/:symbol", get(get_instrument).put(upsert_instrument))
        .route("/api/capabilities", get(get_capabilities))
        .route("/api/debug/providers", get(debug_providers))
        .route("/api/debug/agents", get(debug_agents))
        .route("/api/debug/agents/:role/chat", post(debug_agent_chat))
        .with_state(state)
}

async fn health(State(state): State<Arc<AppState>>) -> Json<serde_json::Value> {
    Json(json!({
        "status": "ok",
        "server": {
            "host": state.config.server.host,
            "port": state.config.server.port,
        },
        "frontend_origin": state.config.server.frontend_origin,
        "providers": state.providers.provider_statuses(),
    }))
}

async fn dashboard(
    State(state): State<Arc<AppState>>,
) -> AppResult<Json<crate::models::DashboardResponse>> {
    info!("[DASHBOARD] GET /api/dashboard — request received");
    let resp = db::build_dashboard(&state.db).await?;
    Ok(Json(resp))
}

async fn agent_status(
    State(state): State<Arc<AppState>>,
) -> AppResult<Json<Vec<crate::models::AgentStatusView>>> {
    info!("[AGENT] GET /api/agents/status — request received");
    let statuses = db::build_agent_statuses(&state.db).await?;
    for s in &statuses {
        info!("[AGENT]   {} ({}) — status={} open_runs={}", s.label, s.role, s.status, s.open_runs);
    }
    Ok(Json(statuses))
}

async fn list_schedules(
    State(state): State<Arc<AppState>>,
) -> AppResult<Json<Vec<crate::models::ScheduleView>>> {
    let items = db::list_schedules(&state.db)
        .await?
        .into_iter()
        .map(db::to_schedule_view)
        .collect();
    Ok(Json(items))
}

async fn create_schedule(
    State(state): State<Arc<AppState>>,
    Json(request): Json<crate::models::CreateScheduleRequest>,
) -> AppResult<(StatusCode, Json<crate::models::ScheduleView>)> {
    info!("[SCHEDULE] POST /api/schedules — name='{}' cron='{}' type='{}'",
        request.name, request.cron_expr, request.job_type);
    if request.name.trim().is_empty()
        || request.cron_expr.trim().is_empty()
        || request.job_type.trim().is_empty()
    {
        return Err(AppError::bad_request(
            "name, cron_expr and job_type are required",
        ));
    }

    let row = db::create_schedule(&state.db, &request).await?;
    let view = db::to_schedule_view(row);
    state.events.publish(
        "job.status",
        &json!({ "schedule": view.name, "status": "created" }),
    );
    Ok((StatusCode::CREATED, Json(view)))
}

async fn update_schedule(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
    Json(request): Json<crate::models::UpdateScheduleRequest>,
) -> AppResult<Json<crate::models::ScheduleView>> {
    ensure_schedule(&state, &id).await?;
    let row = db::update_schedule(&state.db, &id, &request).await?;
    Ok(Json(db::to_schedule_view(row)))
}

async fn delete_schedule(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> AppResult<StatusCode> {
    ensure_schedule(&state, &id).await?;
    db::delete_schedule(&state.db, &id).await?;
    state.events.publish(
        "job.status",
        &json!({ "schedule_id": id, "status": "deleted" }),
    );
    Ok(StatusCode::NO_CONTENT)
}

async fn debug_providers(
    State(state): State<Arc<AppState>>,
) -> AppResult<Json<Vec<ProviderStatusView>>> {
    Ok(Json(state.providers.provider_statuses()))
}

async fn get_capabilities(
    State(state): State<Arc<AppState>>,
) -> AppResult<Json<crate::models::CapabilitiesView>> {
    Ok(Json(state.providers.all_capabilities()))
}

async fn debug_agents(State(state): State<Arc<AppState>>) -> AppResult<Json<Vec<DebugAgentView>>> {
    let statuses = db::build_agent_statuses(&state.db).await?;
    let providers = state.providers.available_provider_names();
    let default_provider = state.providers.default_provider_name();

    let agents = AgentRole::visible_agents()
        .iter()
        .map(|role| {
            let status = statuses
                .iter()
                .find(|item| item.role == role.as_str())
                .map(|item| item.status.clone())
                .unwrap_or_else(|| "idle".to_string());
            let capabilities = state.providers.agent_capabilities(*role);

            DebugAgentView {
                role: role.as_str().to_string(),
                label: role.label().to_string(),
                status,
                providers: providers.clone(),
                default_provider: default_provider.clone(),
                available_commands: capabilities.available_commands,
                mcp_servers: capabilities.mcp_servers,
                native_tools: capabilities.native_tools,
            }
        })
        .collect();

    Ok(Json(agents))
}

async fn debug_agent_chat(
    State(state): State<Arc<AppState>>,
    Path(role): Path<String>,
    Json(request): Json<DebugAgentChatRequest>,
) -> AppResult<Sse<impl Stream<Item = Result<Event, Infallible>>>> {
    info!("[AGENT-CHAT] POST /api/debug/agents/{}/chat — message='{}'", role, &request.message[..request.message.len().min(100)]);
    if request.message.trim().is_empty() {
        return Err(AppError::bad_request("message là bắt buộc"));
    }

    let agent_role = AgentRole::from_str(&role).map_err(AppError::bad_request)?;
    info!("[AGENT-CHAT] Agent role={} label={}", agent_role.as_str(), agent_role.label());
    if !AgentRole::visible_agents().contains(&agent_role) {
        return Err(AppError::bad_request("agent này không hỗ trợ debug chat"));
    }

    let context = None;

    let (tx, rx) = mpsc::unbounded_channel::<ChatStreamEvent>();
    let _ = tx.send(ChatStreamEvent::Connected);

    let providers = state.providers.clone();
    tokio::spawn(async move {
        match providers
            .chat(
                agent_role,
                AgentChatOptions {
                    provider: request.provider,
                    chat_session_id: request.chat_session_id,
                    history: request.history,
                    message: request.message,
                    max_tokens: request.max_tokens,
                    temperature: request.temperature,
                    context,
                    tool_filter: None,
                },
                Some(tx.clone()),
            )
            .await
        {
            Ok(response) => {
                let _ = tx.send(ChatStreamEvent::Response { data: Box::new(response) });
            }
            Err(e) => {
                let _ = tx.send(ChatStreamEvent::Error {
                    message: e.to_string(),
                });
            }
        }
    });

    let stream = UnboundedReceiverStream::new(rx).map(|event| {
        Ok(Event::default().data(serde_json::to_string(&event).unwrap_or_default()))
    });
    Ok(Sse::new(stream).keep_alive(KeepAlive::new().interval(Duration::from_secs(15))))
}

// ─── Schedule SSE Stream ─────────────────────────────────────────────

async fn schedule_stream(
    State(state): State<Arc<AppState>>,
) -> Sse<impl Stream<Item = Result<Event, Infallible>>> {
    let mut rx = state.events.subscribe();

    let stream = async_stream::stream! {
        loop {
            match rx.recv().await {
                Ok(event) if event.event_type == "job.status" => {
                    let sse = Event::default()
                        .event("job_status")
                        .json_data(&event.payload)
                        .unwrap_or_else(|_| Event::default().data("{}"));
                    yield Ok(sse);
                }
                Ok(_) => continue,
                Err(broadcast::error::RecvError::Lagged(n)) => {
                    info!("[SSE] schedule_stream lagged {n} events");
                    continue;
                }
                Err(broadcast::error::RecvError::Closed) => break,
            }
        }
    };

    Sse::new(stream).keep_alive(KeepAlive::new().interval(Duration::from_secs(15)))
}

// ─── Instruments ─────────────────────────────────────────────────────

async fn list_instruments(
    State(state): State<Arc<AppState>>,
) -> AppResult<Json<Vec<crate::models::InstrumentView>>> {
    let items = db::list_instruments(&state.db)
        .await?
        .into_iter()
        .map(db::to_instrument_view)
        .collect();
    Ok(Json(items))
}

async fn get_instrument(
    State(state): State<Arc<AppState>>,
    Path(symbol): Path<String>,
) -> AppResult<Json<crate::models::InstrumentView>> {
    let row = db::get_instrument(&state.db, &symbol)
        .await?
        .ok_or_else(|| AppError::not_found(format!("instrument {symbol} not found")))?;
    Ok(Json(db::to_instrument_view(row)))
}

async fn upsert_instrument(
    State(state): State<Arc<AppState>>,
    Path(symbol): Path<String>,
    Json(request): Json<crate::models::UpsertInstrumentRequest>,
) -> AppResult<Json<crate::models::InstrumentView>> {
    let row = db::upsert_instrument(&state.db, &symbol, &request).await?;
    let view = db::to_instrument_view(row);
    state.events.publish(
        "instrument.updated",
        &json!({ "symbol": symbol }),
    );
    Ok(Json(view))
}

async fn ensure_schedule(state: &AppState, schedule_id: &str) -> AppResult<()> {
    if db::schedule_exists(&state.db, schedule_id).await? {
        Ok(())
    } else {
        Err(AppError::not_found(format!(
            "schedule {schedule_id} not found"
        )))
    }
}

type AppResult<T> = Result<T, AppError>;

#[derive(Debug, thiserror::Error)]
enum AppError {
    #[error("{message}")]
    Status { status: StatusCode, message: String },
    #[error(transparent)]
    Anyhow(#[from] anyhow::Error),
}

impl AppError {
    fn bad_request(message: impl Into<String>) -> Self {
        Self::Status {
            status: StatusCode::BAD_REQUEST,
            message: message.into(),
        }
    }

    fn not_found(message: impl Into<String>) -> Self {
        Self::Status {
            status: StatusCode::NOT_FOUND,
            message: message.into(),
        }
    }
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        match self {
            Self::Status { status, message } => {
                (status, Json(json!({ "error": message }))).into_response()
            }
            Self::Anyhow(error) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({ "error": error.to_string() })),
            )
                .into_response(),
        }
    }
}
