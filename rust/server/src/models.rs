use serde::{Deserialize, Serialize};
use serde_json::Value;
use sqlx::FromRow;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Citation {
    pub source_id: String,
    pub url: String,
    pub title: String,
    pub snippet: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateInvestigationRequest {
    pub topic: String,
    pub goal: Option<String>,
    pub sections: Option<Vec<String>>,
    pub source_scope: Option<String>,
    pub priority: Option<String>,
    pub tags: Option<Vec<String>>,
    pub seed_urls: Option<Vec<String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateScheduleRequest {
    pub name: String,
    pub cron_expr: String,
    pub job_type: String,
    pub enabled: bool,
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
pub struct InvestigationRow {
    pub id: String,
    pub topic: String,
    pub goal: String,
    pub status: String,
    pub source_scope: String,
    pub priority: String,
    pub summary: Option<String>,
    pub final_report: Option<String>,
    pub tags_json: String,
    pub seed_urls_json: String,
    pub created_at: String,
    pub updated_at: String,
    pub completed_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct SectionRow {
    pub id: String,
    pub investigation_id: String,
    pub slug: String,
    pub title: String,
    pub status: String,
    pub conclusion: Option<String>,
    pub position: i64,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct AgentMessageRow {
    pub id: String,
    pub investigation_id: String,
    pub section_id: Option<String>,
    pub agent_role: String,
    pub target_role: Option<String>,
    pub kind: String,
    pub content: String,
    pub citations_json: String,
    pub confidence: Option<f64>,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct FindingRow {
    pub id: String,
    pub investigation_id: String,
    pub section_id: Option<String>,
    pub agent_role: String,
    pub kind: String,
    pub title: String,
    pub summary: String,
    pub direction: Option<String>,
    pub confidence: f64,
    pub evidence_json: String,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct SourceDocumentRow {
    pub id: String,
    pub investigation_id: String,
    pub url: String,
    pub title: String,
    pub fetched_at: String,
    pub excerpt: Option<String>,
    pub metadata_json: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct HeartbeatRow {
    pub component: String,
    pub scope: String,
    pub status_text: String,
    pub last_seen_at: String,
    pub ttl_seconds: i64,
    pub details_json: String,
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
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InvestigationSummary {
    pub id: String,
    pub topic: String,
    pub goal: String,
    pub status: String,
    pub source_scope: String,
    pub priority: String,
    pub summary: Option<String>,
    pub final_report: Option<String>,
    pub tags: Vec<String>,
    pub seed_urls: Vec<String>,
    pub created_at: String,
    pub updated_at: String,
    pub completed_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SectionView {
    pub id: String,
    pub slug: String,
    pub title: String,
    pub status: String,
    pub conclusion: Option<String>,
    pub position: i64,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MessageView {
    pub id: String,
    pub section_id: Option<String>,
    pub agent_role: String,
    pub target_role: Option<String>,
    pub kind: String,
    pub content: String,
    pub citations: Vec<Citation>,
    pub confidence: Option<f64>,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FindingView {
    pub id: String,
    pub section_id: Option<String>,
    pub agent_role: String,
    pub kind: String,
    pub title: String,
    pub summary: String,
    pub direction: Option<String>,
    pub confidence: f64,
    pub evidence: Vec<Citation>,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SourceView {
    pub id: String,
    pub url: String,
    pub title: String,
    pub fetched_at: String,
    pub excerpt: Option<String>,
    pub metadata: Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HeartbeatView {
    pub component: String,
    pub scope: String,
    pub status_text: String,
    pub health: String,
    pub last_seen_at: String,
    pub ttl_seconds: i64,
    pub details: Value,
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
pub struct DashboardStats {
    pub total_investigations: i64,
    pub running_investigations: i64,
    pub completed_investigations: i64,
    pub recent_findings: i64,
    pub stale_heartbeats: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DashboardResponse {
    pub stats: DashboardStats,
    pub recent_investigations: Vec<InvestigationSummary>,
    pub recent_findings: Vec<FindingView>,
    pub agent_statuses: Vec<AgentStatusView>,
    pub schedules: Vec<ScheduleView>,
    pub heartbeats: Vec<HeartbeatView>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InvestigationDetail {
    pub investigation: InvestigationSummary,
    pub sections: Vec<SectionView>,
    pub transcript: Vec<MessageView>,
    pub findings: Vec<FindingView>,
    pub sources: Vec<SourceView>,
    pub heartbeats: Vec<HeartbeatView>,
}

pub fn new_id() -> String {
    Uuid::new_v4().to_string()
}
