use std::str::FromStr;
use std::sync::Arc;

use anyhow::Result;
use serde_json::json;
use tokio::time::{interval, Duration};
use tracing::{error, info, warn};

use crate::{
    agents::{AgentChatOptions, AgentRole},
    db, AppState,
};

pub fn start_background_workers(state: Arc<AppState>) {
    start_scheduler_loop(state);
}

fn start_scheduler_loop(state: Arc<AppState>) {
    tokio::spawn(async move {
        let mut ticker = interval(Duration::from_secs(
            state.config.scheduler.interval_seconds,
        ));

        loop {
            ticker.tick().await;

            let schedules = match db::due_schedules(&state.db).await {
                Ok(items) => items,
                Err(error) => {
                    error!(error = %error, "failed to load due schedules");
                    continue;
                }
            };

            for schedule in schedules {
                info!(
                    "[SCHEDULER] Executing schedule '{}' (type={}, agent={}, msg={})",
                    schedule.name,
                    schedule.job_type,
                    schedule.agent_role,
                    &schedule.message[..schedule.message.len().min(60)]
                );

                if let Err(error) = execute_schedule(state.clone(), &schedule).await {
                    error!(schedule = %schedule.name, error = %error, "schedule execution failed");
                }

                if let Err(error) = db::mark_schedule_executed(&state.db, &schedule).await {
                    error!(schedule = %schedule.name, error = %error, "failed to mark schedule as executed");
                }
            }
        }
    });
}

async fn execute_schedule(
    state: Arc<AppState>,
    schedule: &crate::models::ScheduleRow,
) -> Result<()> {
    // Mark as running
    if let Err(error) = db::update_schedule_status(&state.db, &schedule.id, "running", None).await {
        warn!(schedule = %schedule.name, error = %error, "failed to set running status");
    }

    let agent_role = match AgentRole::from_str(&schedule.agent_role) {
        Ok(role) => role,
        Err(err) => {
            let msg = format!("invalid agent_role '{}': {}", schedule.agent_role, err);
            db::update_schedule_status(&state.db, &schedule.id, "failed", Some(&msg)).await?;
            state.events.publish(
                "job.status",
                &json!({ "schedule": schedule.name, "status": "failed", "error": msg }),
            );
            return Ok(());
        }
    };

    let message = if schedule.message.trim().is_empty() {
        format!("Scheduled task: {}", schedule.name)
    } else {
        schedule.message.clone()
    };

    info!(
        "[SCHEDULER] Sending to agent {} — '{}'",
        agent_role.as_str(),
        &message[..message.len().min(80)]
    );

    let result = state
        .providers
        .chat(
            agent_role,
            AgentChatOptions {
                provider: None,
                chat_session_id: None,
                history: vec![],
                message,
                max_tokens: None,
                temperature: None,
                context: None,
            },
            None,
        )
        .await;

    match result {
        Ok(response) => {
            let truncated = if response.content.len() > 500 {
                format!("{}...", &response.content[..500])
            } else {
                response.content.clone()
            };

            db::update_schedule_status(
                &state.db,
                &schedule.id,
                "completed",
                Some(&truncated),
            )
            .await?;

            state.events.publish(
                "job.status",
                &json!({
                    "schedule": schedule.name,
                    "status": "completed",
                    "agent_role": agent_role.as_str(),
                    "result_preview": &truncated[..truncated.len().min(200)],
                }),
            );

            info!(
                "[SCHEDULER] Schedule '{}' completed — {} chars response",
                schedule.name,
                response.content.len()
            );
        }
        Err(error) => {
            let err_msg = format!("{error:#}");
            let truncated = if err_msg.len() > 500 {
                format!("{}...", &err_msg[..500])
            } else {
                err_msg.clone()
            };

            db::update_schedule_status(&state.db, &schedule.id, "failed", Some(&truncated)).await?;

            state.events.publish(
                "job.status",
                &json!({
                    "schedule": schedule.name,
                    "status": "failed",
                    "error": truncated,
                }),
            );

            error!(
                schedule = %schedule.name,
                error = %err_msg,
                "agent chat failed for scheduled task"
            );
        }
    }

    Ok(())
}
