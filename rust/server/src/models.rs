use serde::{Deserialize, Serialize};
use serde_json::Value;
use sqlx::FromRow;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateScheduleRequest {
    pub name: String,
    pub cron_expr: String,
    pub job_type: String,
    pub enabled: bool,
    pub agent_role: String,
    pub message: String,
    #[serde(default)]
    pub payload: Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateScheduleRequest {
    pub cron_expr: Option<String>,
    pub enabled: Option<bool>,
    pub payload: Option<Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct ScheduleRow {
    pub id: String,
    pub name: String,
    pub cron_expr: String,
    pub job_type: String,
    pub enabled: i64,
    pub payload_json: String,
    pub last_run_at: Option<String>,
    pub next_run_at: Option<String>,
    pub updated_at: String,
    pub agent_role: String,
    pub message: String,
    pub last_status: String,
    pub last_result: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScheduleView {
    pub id: String,
    pub name: String,
    pub cron_expr: String,
    pub job_type: String,
    pub enabled: bool,
    pub payload: Value,
    pub last_run_at: Option<String>,
    pub next_run_at: Option<String>,
    pub updated_at: String,
    pub agent_role: String,
    pub message: String,
    pub last_status: String,
    pub last_result: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentStatusView {
    pub role: String,
    pub label: String,
    pub status: String,
    pub last_seen_at: Option<String>,
    pub last_message: Option<String>,
    pub open_runs: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DashboardResponse {
    pub agent_statuses: Vec<AgentStatusView>,
    pub schedules: Vec<ScheduleView>,
}

pub fn new_id() -> String {
    Uuid::new_v4().to_string()
}
