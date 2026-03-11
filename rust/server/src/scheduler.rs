use std::sync::Arc;

use anyhow::Result;
use serde_json::json;
use tokio::time::{interval, Duration};
use tracing::error;

use crate::{db, runner, AppState};

pub fn start_background_workers(state: Arc<AppState>) {
    start_service_heartbeat(state.clone());
    start_scheduler_loop(state);
}

fn start_service_heartbeat(state: Arc<AppState>) {
    tokio::spawn(async move {
        let mut ticker = interval(Duration::from_secs(
            state.config.heartbeat.api_interval_seconds,
        ));
        loop {
            ticker.tick().await;
            if let Err(error) = db::upsert_heartbeat(
                &state.db,
                "service",
                "server",
                "healthy",
                state.config.heartbeat.api_ttl_seconds,
                json!({ "status": "online" }),
            )
            .await
            {
                error!(error = %error, "service heartbeat failed");
                continue;
            }

            state.events.publish(
                "heartbeat",
                None,
                &json!({ "component": "service", "scope": "server", "status_text": "healthy" }),
            );
        }
    });
}

fn start_scheduler_loop(state: Arc<AppState>) {
    tokio::spawn(async move {
        let mut ticker = interval(Duration::from_secs(
            state.config.heartbeat.scheduler_interval_seconds,
        ));

        loop {
            ticker.tick().await;

            if let Err(error) = db::upsert_heartbeat(
                &state.db,
                "service",
                "scheduler",
                "healthy",
                state.config.heartbeat.scheduler_ttl_seconds,
                json!({ "interval_seconds": state.config.heartbeat.scheduler_interval_seconds }),
            )
            .await
            {
                error!(error = %error, "scheduler heartbeat failed");
            }

            let schedules = match db::due_schedules(&state.db).await {
                Ok(items) => items,
                Err(error) => {
                    error!(error = %error, "failed to load due schedules");
                    continue;
                }
            };

            for schedule in schedules {
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
    match schedule.job_type.as_str() {
        "heartbeat_sweep" => {
            let swept = db::sweep_stale_heartbeats(&state.db).await?;
            state.events.publish(
                "job.status",
                None,
                &json!({ "schedule": schedule.name, "status": "completed", "swept": swept }),
            );
        }
        "memory_compaction" => {
            let removed = db::compact_history(&state.db).await?;
            state.events.publish(
                "job.status",
                None,
                &json!({ "schedule": schedule.name, "status": "completed", "removed": removed }),
            );
        }
        "investigation_refresh" => {
            if let Some(investigation_id) = db::from_json_value(&schedule.payload_json)
                .get("investigation_id")
                .and_then(|value| value.as_str())
            {
                runner::spawn_investigation(state.clone(), investigation_id.to_string());
                state.events.publish(
                    "job.status",
                    Some(investigation_id.to_string()),
                    &json!({ "schedule": schedule.name, "status": "spawned" }),
                );
            }
        }
        other => {
            state.events.publish(
                "job.status",
                None,
                &json!({ "schedule": schedule.name, "status": "skipped", "reason": format!("unsupported job_type {other}") }),
            );
        }
    }

    Ok(())
}
