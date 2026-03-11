use std::{convert::Infallible, str::FromStr, sync::Arc};

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
use futures_util::{stream, Stream, StreamExt};
use serde_json::json;
use tokio_stream::wrappers::BroadcastStream;

use crate::{
    db,
    models::{AgentRole, DebugAgentView},
    providers::{AgentChatOptions, AgentPromptContext},
    AppState,
};

pub fn router(state: Arc<AppState>) -> Router {
    Router::new()
        .route("/health", get(health))
        .route("/api/dashboard", get(dashboard))
        .route(
            "/api/investigations",
            get(list_investigations).post(create_investigation),
        )
        .route("/api/investigations/:id", get(get_investigation))
        .route("/api/investigations/:id/stream", get(stream_investigation))
        .route("/api/agents/status", get(agent_status))
        .route("/api/heartbeats", get(heartbeats))
        .route("/api/schedules", get(list_schedules).post(create_schedule))
        .route("/api/schedules/:id", patch(update_schedule))
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
    Ok(Json(db::build_dashboard(&state.db).await?))
}

async fn list_investigations(
    State(state): State<Arc<AppState>>,
) -> AppResult<Json<Vec<crate::models::InvestigationSummary>>> {
    let items = db::list_investigation_rows(&state.db)
        .await?
        .into_iter()
        .map(db::to_investigation_summary)
        .collect();
    Ok(Json(items))
}

async fn create_investigation(
    State(state): State<Arc<AppState>>,
    Json(request): Json<crate::models::CreateInvestigationRequest>,
) -> AppResult<(StatusCode, Json<crate::models::InvestigationDetail>)> {
    if request.topic.trim().is_empty() {
        return Err(AppError::bad_request("topic is required"));
    }

    let row = db::create_investigation(&state.db, &state.config, request).await?;
    let detail = db::build_investigation_detail(&state.db, &row.id).await?;
    state.events.publish(
        "investigation.updated",
        Some(row.id.clone()),
        &detail.investigation,
    );
    Ok((StatusCode::CREATED, Json(detail)))
}

async fn get_investigation(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> AppResult<Json<crate::models::InvestigationDetail>> {
    ensure_investigation(&state, &id).await?;
    Ok(Json(db::build_investigation_detail(&state.db, &id).await?))
}

async fn agent_status(
    State(state): State<Arc<AppState>>,
) -> AppResult<Json<Vec<crate::models::AgentStatusView>>> {
    Ok(Json(db::build_agent_statuses(&state.db).await?))
}

async fn heartbeats(
    State(state): State<Arc<AppState>>,
) -> AppResult<Json<Vec<crate::models::HeartbeatView>>> {
    let items = db::list_heartbeats(&state.db)
        .await?
        .into_iter()
        .map(db::to_heartbeat_view)
        .collect();
    Ok(Json(items))
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
        None,
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

async fn debug_providers(
    State(state): State<Arc<AppState>>,
) -> AppResult<Json<Vec<crate::models::ProviderStatusView>>> {
    Ok(Json(state.providers.provider_statuses()))
}

async fn debug_agents(State(state): State<Arc<AppState>>) -> AppResult<Json<Vec<DebugAgentView>>> {
    let statuses = db::build_agent_statuses(&state.db).await?;
    let providers = state.providers.available_provider_names();
    let default_provider = state.providers.default_provider_name();

    let agents = AgentRole::team()
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
                common_skills: capabilities.common_skills,
                agent_skills: capabilities.agent_skills,
                skill_tools: capabilities.skill_tools,
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
    Json(request): Json<crate::models::DebugAgentChatRequest>,
) -> AppResult<Json<crate::models::DebugAgentChatResponse>> {
    if request.message.trim().is_empty() {
        return Err(AppError::bad_request("message là bắt buộc"));
    }

    let agent_role = AgentRole::from_str(&role).map_err(AppError::bad_request)?;
    let context = if request.include_backend_context.unwrap_or(true) {
        load_backend_context(&state, request.investigation_id.as_deref()).await?
    } else {
        None
    };

    let response = state
        .providers
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
            },
        )
        .await?;

    Ok(Json(response))
}

async fn stream_investigation(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> AppResult<Sse<impl Stream<Item = Result<Event, Infallible>>>> {
    ensure_investigation(&state, &id).await?;

    let receiver = state.events.subscribe();
    let stream_id = id.clone();
    let live_stream = BroadcastStream::new(receiver).filter_map(move |item| {
        let target = stream_id.clone();
        async move {
            match item.ok() {
                Some(event) if event.investigation_id.as_deref() == Some(target.as_str()) => {
                    let payload = serde_json::to_string(&event).ok()?;
                    Some(Ok(Event::default().event(event.event_type).data(payload)))
                }
                _ => None,
            }
        }
    });

    let init = stream::once(async move {
        Ok(Event::default().event("heartbeat").data(
            json!({
                "event_type": "heartbeat",
                "investigation_id": id,
                "payload": { "message": "stream-connected" },
                "timestamp": db::now_rfc3339(),
            })
            .to_string(),
        ))
    });

    Ok(Sse::new(init.chain(live_stream))
        .keep_alive(KeepAlive::new().interval(std::time::Duration::from_secs(10))))
}

async fn load_backend_context(
    state: &AppState,
    investigation_id: Option<&str>,
) -> AppResult<Option<AgentPromptContext>> {
    let Some(investigation_id) = investigation_id else {
        return Ok(None);
    };

    ensure_investigation(state, investigation_id).await?;
    let detail = db::build_investigation_detail(&state.db, investigation_id).await?;

    let findings = detail
        .findings
        .iter()
        .take(3)
        .map(|finding| {
            format!(
                "- {} [{}]: {}",
                finding.title,
                finding
                    .direction
                    .clone()
                    .unwrap_or_else(|| finding.kind.clone()),
                finding.summary
            )
        })
        .collect::<Vec<_>>()
        .join("\n");

    let sections = detail
        .sections
        .iter()
        .map(|section| {
            format!(
                "- {} ({}): {}",
                section.title,
                section.status,
                section
                    .conclusion
                    .clone()
                    .unwrap_or_else(|| "Chưa có".to_string())
            )
        })
        .collect::<Vec<_>>()
        .join("\n");

    let preview = format!(
        "Investigation: {}\nStatus: {}\nGoal: {}\nSummary: {}\n\nSections:\n{}\n\nFindings:\n{}",
        detail.investigation.topic,
        detail.investigation.status,
        detail.investigation.goal,
        detail
            .investigation
            .summary
            .clone()
            .unwrap_or_else(|| "Chưa có".to_string()),
        truncate_text(&sections, 1400),
        if findings.is_empty() {
            "Chưa có findings".to_string()
        } else {
            truncate_text(&findings, 1400)
        }
    );

    Ok(Some(AgentPromptContext {
        investigation_id: Some(investigation_id.to_string()),
        preview: Some(preview),
    }))
}

fn truncate_text(value: &str, max_chars: usize) -> String {
    if value.chars().count() <= max_chars {
        return value.to_string();
    }

    value.chars().take(max_chars).collect::<String>() + "..."
}

async fn ensure_investigation(state: &AppState, investigation_id: &str) -> AppResult<()> {
    if db::investigation_exists(&state.db, investigation_id).await? {
        Ok(())
    } else {
        Err(AppError::not_found(format!(
            "investigation {investigation_id} not found"
        )))
    }
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
