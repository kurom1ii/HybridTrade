use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::PathBuf;
use std::str::FromStr;
use std::sync::Arc;

use anyhow::Result;
use serde_json::json;
use tokio::time::{interval, Duration};
use tracing::{error, info, warn};

use crate::{
    agents::{AgentChatOptions, AgentRole},
    db,
    models::ToolFilter,
    AppState,
};

const DEFAULT_TASK_LOG_PATH: &str = "./logs/agent-task-responses.log";

/// Truncate a string to at most `max_bytes` without splitting a multi-byte
/// UTF-8 character.  Returns a `&str` that is always valid UTF-8.
fn safe_truncate(s: &str, max_bytes: usize) -> &str {
    if s.len() <= max_bytes {
        return s;
    }
    let mut end = max_bytes;
    while !s.is_char_boundary(end) && end > 0 {
        end -= 1;
    }
    &s[..end]
}

pub fn start_background_workers(state: Arc<AppState>) {
    start_scheduler_loop(state);
}

fn resolve_task_log_path() -> PathBuf {
    std::env::var("HYBRIDTRADE_TASK_LOG_PATH")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from(DEFAULT_TASK_LOG_PATH))
}

fn append_task_log(
    schedule_name: &str,
    agent_role: &str,
    provider: &str,
    model: &str,
    message: &str,
    status: &str,
    content: &str,
    tool_calls: &[String],
) {
    let path = resolve_task_log_path();
    if let Some(parent) = path.parent() {
        let _ = fs::create_dir_all(parent);
    }

    let file = OpenOptions::new().create(true).append(true).open(&path);
    let mut file = match file {
        Ok(f) => f,
        Err(err) => {
            warn!(error = %err, path = %path.display(), "cannot open agent task log");
            return;
        }
    };

    let timestamp = chrono::Local::now().to_rfc3339();
    let tool_summary = if tool_calls.is_empty() {
        "(none)".to_string()
    } else {
        tool_calls.join(", ")
    };

    let _ = writeln!(file, "===== agent_task_response =====");
    let _ = writeln!(file, "timestamp: {timestamp}");
    let _ = writeln!(file, "schedule: {schedule_name}");
    let _ = writeln!(file, "agent_role: {agent_role}");
    let _ = writeln!(file, "provider: {provider}");
    let _ = writeln!(file, "model: {model}");
    let _ = writeln!(file, "status: {status}");
    let _ = writeln!(file, "tool_calls: [{tool_summary}]");
    let _ = writeln!(file, "message:");
    let _ = writeln!(file, "{message}");
    let _ = writeln!(file, "response:");
    let _ = writeln!(file, "{content}");
    let _ = writeln!(file, "===== end_agent_task_response =====");
    let _ = writeln!(file);
    let _ = file.flush();
}

fn start_scheduler_loop(state: Arc<AppState>) {
    let interval_secs = state.config.scheduler.interval_seconds;
    info!("[SCHEDULER] Started — polling every {interval_secs}s");

    tokio::spawn(async move {
        let mut ticker = interval(Duration::from_secs(interval_secs));

        loop {
            ticker.tick().await;

            let schedules = match db::due_schedules(&state.db).await {
                Ok(items) => items,
                Err(error) => {
                    error!(error = %error, "failed to load due schedules");
                    continue;
                }
            };

            if schedules.is_empty() {
                continue;
            }

            info!("[SCHEDULER] Found {} due schedule(s)", schedules.len());

            for schedule in schedules {
                // ── Advance next_run_at IMMEDIATELY so the schedule won't be
                //    picked up again while the agent call is in flight.
                if let Err(error) = db::mark_schedule_executed(&state.db, &schedule).await {
                    error!(schedule = %schedule.name, error = %error, "failed to advance schedule");
                    continue;
                }

                if schedule.job_type != "agent_task" {
                    info!(
                        "[SCHEDULER] Skipping schedule '{}' — job_type='{}' is not 'agent_task'",
                        schedule.name, schedule.job_type
                    );
                    continue;
                }

                if schedule.message.trim().is_empty() {
                    warn!(
                        "[SCHEDULER] Schedule '{}' has empty message — skipping agent call",
                        schedule.name
                    );
                    let _ = db::update_schedule_status(
                        &state.db,
                        &schedule.id,
                        "failed",
                        Some("message is empty — set a message for the agent"),
                    )
                    .await;
                    continue;
                }

                info!(
                    "[SCHEDULER] >>> Dispatching '{}' — agent={} msg='{}'",
                    schedule.name,
                    schedule.agent_role,
                    safe_truncate(&schedule.message, 80)
                );

                // ── Spawn each task concurrently — no task waits for another.
                let task_state = state.clone();
                let task_schedule = schedule.clone();
                tokio::spawn(async move {
                    if let Err(error) =
                        execute_schedule(task_state, &task_schedule).await
                    {
                        error!(
                            schedule = %task_schedule.name,
                            error = %error,
                            "schedule execution failed"
                        );
                    }
                });
            }
        }
    });
}

async fn execute_schedule(
    state: Arc<AppState>,
    schedule: &crate::models::ScheduleRow,
) -> Result<()> {
    // Mark as running
    if let Err(error) =
        db::update_schedule_status(&state.db, &schedule.id, "running", None).await
    {
        warn!(schedule = %schedule.name, error = %error, "failed to set running status");
    }

    let agent_role = match AgentRole::from_str(&schedule.agent_role) {
        Ok(role) => role,
        Err(err) => {
            let msg = format!("invalid agent_role '{}': {}", schedule.agent_role, err);
            error!("[SCHEDULER] {msg}");
            db::update_schedule_status(&state.db, &schedule.id, "failed", Some(&msg)).await?;

            append_task_log(
                &schedule.name,
                &schedule.agent_role,
                "-",
                "-",
                &schedule.message,
                "failed",
                &msg,
                &[],
            );

            state.events.publish(
                "job.status",
                &json!({ "schedule": schedule.name, "status": "failed", "error": msg }),
            );
            return Ok(());
        }
    };

    info!(
        "[SCHEDULER] Calling providers.chat() — role={} message='{}'",
        agent_role.as_str(),
        safe_truncate(&schedule.message, 100)
    );

    let tool_filter = ToolFilter {
        allowed_tools: schedule
            .allowed_tools
            .as_deref()
            .filter(|s| !s.is_empty() && *s != "null")
            .and_then(|s| serde_json::from_str(s).ok()),
        allowed_mcps: schedule
            .allowed_mcps
            .as_deref()
            .filter(|s| !s.is_empty() && *s != "null")
            .and_then(|s| serde_json::from_str(s).ok()),
        skills: schedule
            .skills
            .as_deref()
            .filter(|s| !s.is_empty() && *s != "null")
            .and_then(|s| serde_json::from_str::<Vec<String>>(s).ok())
            .unwrap_or_default(),
    };

    let result = state
        .providers
        .chat(
            agent_role,
            AgentChatOptions {
                provider: None,
                chat_session_id: None,
                history: vec![],
                message: schedule.message.clone(),
                max_tokens: None,
                temperature: None,
                context: None,
                tool_filter: Some(tool_filter),
            },
            None,
        )
        .await;

    match result {
        Ok(response) => {
            let tool_names: Vec<String> = response
                .debug
                .tool_calls
                .iter()
                .map(|tc| format!("{} [{}]", tc.name, tc.status))
                .collect();

            // Store structured JSON with full detail for frontend consumption
            let result_json = json!({
                "content": safe_truncate(&response.content, 8000),
                "provider": response.provider,
                "model": response.model,
                "tool_calls": response.debug.tool_calls,
                "system_prompt": &response.debug.system_prompt,
                "available_tools": response.debug.available_tools,
                "history_count": response.debug.history_count,
            });
            let result_string = serde_json::to_string(&result_json).unwrap_or_default();

            db::update_schedule_status(
                &state.db,
                &schedule.id,
                "completed",
                Some(&result_string),
            )
            .await?;

            append_task_log(
                &schedule.name,
                agent_role.as_str(),
                &response.provider,
                &response.model,
                &schedule.message,
                "completed",
                &response.content,
                &tool_names,
            );

            let content_preview = if response.content.len() > 200 {
                format!("{}...", safe_truncate(&response.content, 200))
            } else {
                response.content.clone()
            };

            state.events.publish(
                "job.status",
                &json!({
                    "schedule": schedule.name,
                    "status": "completed",
                    "agent_role": agent_role.as_str(),
                    "provider": response.provider,
                    "model": response.model,
                    "tool_calls": tool_names.len(),
                    "result_preview": content_preview,
                }),
            );

            info!(
                "[SCHEDULER] <<< '{}' completed — provider={} model={} tools={} chars={}",
                schedule.name,
                response.provider,
                response.model,
                tool_names.len(),
                response.content.len(),
            );
            if !tool_names.is_empty() {
                info!("[SCHEDULER] Tool calls: [{}]", tool_names.join(", "));
            }
            info!(
                "[SCHEDULER] Agent response:\n─────────────────────────────\n{}\n─────────────────────────────",
                content_preview
            );
        }
        Err(error) => {
            let err_msg = format!("{error:#}");
            let truncated = if err_msg.len() > 500 {
                format!("{}...", safe_truncate(&err_msg, 500))
            } else {
                err_msg.clone()
            };

            db::update_schedule_status(&state.db, &schedule.id, "failed", Some(&truncated))
                .await?;

            append_task_log(
                &schedule.name,
                agent_role.as_str(),
                "-",
                "-",
                &schedule.message,
                "failed",
                &err_msg,
                &[],
            );

            state.events.publish(
                "job.status",
                &json!({
                    "schedule": schedule.name,
                    "status": "failed",
                    "error": truncated,
                }),
            );

            error!(
                "[SCHEDULER] <<< '{}' FAILED — {}",
                schedule.name, err_msg
            );
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::safe_truncate;

    #[test]
    fn truncate_ascii() {
        assert_eq!(safe_truncate("hello world", 5), "hello");
    }

    #[test]
    fn truncate_multibyte() {
        // '—' is 3 bytes (E2 80 94).  Slicing at byte 4 would be inside the
        // second '—', so safe_truncate must back up to byte 3.
        let s = "—— hello";
        assert_eq!(safe_truncate(s, 4), "—");
        assert_eq!(safe_truncate(s, 6), "——");
    }

    #[test]
    fn truncate_emoji() {
        let s = "👋✨ test";
        // 👋 = 4 bytes, ✨ = 3 bytes
        assert_eq!(safe_truncate(s, 4), "👋");
        assert_eq!(safe_truncate(s, 5), "👋");
        assert_eq!(safe_truncate(s, 7), "👋✨");
    }

    #[test]
    fn truncate_no_op_when_short() {
        assert_eq!(safe_truncate("hi", 100), "hi");
    }
}
