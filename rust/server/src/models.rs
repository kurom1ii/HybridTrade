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
    #[serde(default)]
    pub allowed_tools: Option<Vec<String>>,
    #[serde(default)]
    pub allowed_mcps: Option<Vec<String>>,
    #[serde(default)]
    pub skills: Option<Vec<String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateScheduleRequest {
    pub name: Option<String>,
    pub cron_expr: Option<String>,
    pub enabled: Option<bool>,
    pub agent_role: Option<String>,
    pub message: Option<String>,
    pub payload: Option<Value>,
    /// None = keep current; Some(Null) = allow all; Some(Array) = filter list
    pub allowed_tools: Option<Value>,
    pub allowed_mcps: Option<Value>,
    pub skills: Option<Value>,
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
    pub allowed_tools: Option<String>,
    pub allowed_mcps: Option<String>,
    pub skills: Option<String>,
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
    pub allowed_tools: Option<Vec<String>>,
    pub allowed_mcps: Option<Vec<String>>,
    pub skills: Option<Vec<String>>,
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

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct InstrumentRow {
    pub symbol: String,
    pub name: String,
    pub category: String,
    pub direction: String,
    pub confidence: f64,
    pub price: f64,
    pub change_pct: f64,
    pub timeframe: String,
    pub analysis: String,
    pub key_levels: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InstrumentView {
    pub symbol: String,
    pub name: String,
    pub category: String,
    pub direction: String,
    pub confidence: f64,
    pub price: f64,
    pub change_pct: f64,
    pub timeframe: String,
    pub analysis: String,
    pub key_levels: Value,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpsertInstrumentRequest {
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    pub category: Option<String>,
    #[serde(default)]
    pub direction: Option<String>,
    #[serde(default)]
    pub confidence: Option<f64>,
    #[serde(default)]
    pub price: Option<f64>,
    #[serde(default)]
    pub change_pct: Option<f64>,
    #[serde(default)]
    pub timeframe: Option<String>,
    #[serde(default)]
    pub analysis: Option<String>,
    #[serde(default)]
    pub key_levels: Option<Value>,
}

pub fn new_id() -> String {
    Uuid::new_v4().to_string()
}

/// Per-schedule filter controlling which tools/MCPs/skills an agent task can use.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ToolFilter {
    /// None = allow all native tools; Some(vec) = only these
    pub allowed_tools: Option<Vec<String>>,
    /// None = allow all MCP servers; Some(vec) = only these
    pub allowed_mcps: Option<Vec<String>>,
    /// Skills to inject into the agent turn
    #[serde(default)]
    pub skills: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CapabilitiesView {
    pub tools: Vec<String>,
    pub mcps: Vec<String>,
    pub skills: Vec<String>,
}
