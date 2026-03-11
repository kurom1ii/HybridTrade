use anyhow::{anyhow, Context, Result};
use chrono::{DateTime, Duration, SecondsFormat, Utc};
use cron::Schedule;
use serde::Serialize;
use serde_json::Value;
use sqlx::{query, query_as, SqlitePool};

use crate::{
    config::{ConfigBundle, ScheduleSeed},
    models::{
        new_id, AgentMessageRow, AgentRole, AgentStatusView, CreateInvestigationRequest,
        DashboardResponse, DashboardStats, FindingRow, FindingView, HeartbeatRow, HeartbeatView,
        InvestigationDetail, InvestigationRow, InvestigationSummary, ScheduleRow, ScheduleView,
        SectionRow, SectionView, SourceDocumentRow, SourceView,
    },
};

pub fn now_rfc3339() -> String {
    Utc::now().to_rfc3339_opts(SecondsFormat::Secs, true)
}

pub fn parse_rfc3339(value: &str) -> Result<DateTime<Utc>> {
    Ok(DateTime::parse_from_rfc3339(value)
        .with_context(|| format!("invalid datetime {value}"))?
        .with_timezone(&Utc))
}

pub fn to_json_string<T: Serialize + ?Sized>(value: &T) -> String {
    serde_json::to_string(value).unwrap_or_else(|_| "null".to_string())
}

pub fn from_json_vec<T: serde::de::DeserializeOwned>(value: &str) -> Vec<T> {
    serde_json::from_str(value).unwrap_or_default()
}

pub fn from_json_value(value: &str) -> Value {
    serde_json::from_str(value).unwrap_or(Value::Null)
}

pub fn schedule_next_run(cron_expr: &str) -> Result<String> {
    let schedule = cron_expr.parse::<Schedule>()?;
    let next = schedule
        .upcoming(Utc)
        .next()
        .ok_or_else(|| anyhow!("schedule does not yield future runs"))?;
    Ok(next.to_rfc3339_opts(SecondsFormat::Secs, true))
}

pub async fn bootstrap_schedules(pool: &SqlitePool, schedules: &[ScheduleSeed]) -> Result<()> {
    for schedule in schedules {
        let next_run_at = schedule_next_run(&schedule.cron_expr)?;
        query(
            r#"
            INSERT INTO schedules (id, name, cron_expr, job_type, enabled, payload_json, last_run_at, next_run_at, updated_at)
            VALUES (?1, ?2, ?3, ?4, ?5, ?6, NULL, ?7, ?8)
            ON CONFLICT(name) DO UPDATE SET
              cron_expr = excluded.cron_expr,
              job_type = excluded.job_type,
              enabled = excluded.enabled,
              payload_json = excluded.payload_json,
              next_run_at = excluded.next_run_at,
              updated_at = excluded.updated_at
            "#,
        )
        .bind(new_id())
        .bind(&schedule.name)
        .bind(&schedule.cron_expr)
        .bind(&schedule.job_type)
        .bind(if schedule.enabled { 1 } else { 0 })
        .bind(to_json_string(&schedule.payload))
        .bind(next_run_at)
        .bind(now_rfc3339())
        .execute(pool)
        .await?;
    }

    Ok(())
}

pub async fn create_investigation(
    pool: &SqlitePool,
    config: &ConfigBundle,
    request: CreateInvestigationRequest,
) -> Result<InvestigationRow> {
    let now = now_rfc3339();
    let id = new_id();
    let goal = request
        .goal
        .filter(|value| !value.trim().is_empty())
        .unwrap_or_else(|| config.orchestration.default_goal.clone());
    let source_scope = request
        .source_scope
        .filter(|value| !value.trim().is_empty())
        .unwrap_or_else(|| config.orchestration.default_source_scope.clone());
    let priority = request
        .priority
        .filter(|value| !value.trim().is_empty())
        .unwrap_or_else(|| config.orchestration.default_priority.clone());
    let tags = request.tags.unwrap_or_default();
    let seed_urls = request
        .seed_urls
        .filter(|urls| !urls.is_empty())
        .unwrap_or_else(|| config.orchestration.seed_urls.clone());

    query(
        r#"
        INSERT INTO investigations (id, topic, goal, status, source_scope, priority, summary, final_report, tags_json, seed_urls_json, created_at, updated_at, completed_at)
        VALUES (?1, ?2, ?3, 'queued', ?4, ?5, NULL, NULL, ?6, ?7, ?8, ?8, NULL)
        "#,
    )
    .bind(&id)
    .bind(&request.topic)
    .bind(&goal)
    .bind(&source_scope)
    .bind(&priority)
    .bind(to_json_string(&tags))
    .bind(to_json_string(&seed_urls))
    .bind(&now)
    .execute(pool)
    .await?;

    let sections = request
        .sections
        .filter(|items| !items.is_empty())
        .unwrap_or_else(|| config.orchestration.default_sections.clone());

    for (position, title) in sections.iter().enumerate() {
        query(
            r#"
            INSERT INTO investigation_sections (id, investigation_id, slug, title, status, conclusion, position, updated_at)
            VALUES (?1, ?2, ?3, ?4, 'pending', NULL, ?5, ?6)
            "#,
        )
        .bind(new_id())
        .bind(&id)
        .bind(slugify_section(title, position))
        .bind(title)
        .bind(position as i64)
        .bind(&now)
        .execute(pool)
        .await?;
    }

    get_investigation_row(pool, &id).await
}

fn slugify_section(title: &str, position: usize) -> String {
    let lower = title.to_lowercase();
    if lower.contains("nguon") || lower.contains("scope") {
        "source_scope".to_string()
    } else if lower.contains("tin hieu") || lower.contains("technical") {
        "technical_signals".to_string()
    } else if lower.contains("mau thuan") || lower.contains("risk") || lower.contains("rui ro") {
        "contradictions".to_string()
    } else if lower.contains("tong hop") || lower.contains("final") {
        "final_synthesis".to_string()
    } else {
        format!("section_{}", position + 1)
    }
}

pub async fn investigation_exists(pool: &SqlitePool, investigation_id: &str) -> Result<bool> {
    let row = query_as::<_, (i64,)>("SELECT COUNT(1) FROM investigations WHERE id = ?1")
        .bind(investigation_id)
        .fetch_one(pool)
        .await?;
    Ok(row.0 > 0)
}

pub async fn schedule_exists(pool: &SqlitePool, schedule_id: &str) -> Result<bool> {
    let row = query_as::<_, (i64,)>("SELECT COUNT(1) FROM schedules WHERE id = ?1")
        .bind(schedule_id)
        .fetch_one(pool)
        .await?;
    Ok(row.0 > 0)
}

pub async fn get_investigation_row(
    pool: &SqlitePool,
    investigation_id: &str,
) -> Result<InvestigationRow> {
    query_as::<_, InvestigationRow>("SELECT * FROM investigations WHERE id = ?1")
        .bind(investigation_id)
        .fetch_one(pool)
        .await
        .with_context(|| format!("investigation {investigation_id} not found"))
}

pub async fn list_investigation_rows(pool: &SqlitePool) -> Result<Vec<InvestigationRow>> {
    query_as::<_, InvestigationRow>("SELECT * FROM investigations ORDER BY updated_at DESC")
        .fetch_all(pool)
        .await
        .context("cannot list investigations")
}

pub async fn get_sections(pool: &SqlitePool, investigation_id: &str) -> Result<Vec<SectionRow>> {
    query_as::<_, SectionRow>(
        "SELECT * FROM investigation_sections WHERE investigation_id = ?1 ORDER BY position ASC",
    )
    .bind(investigation_id)
    .fetch_all(pool)
    .await
    .context("cannot list sections")
}

pub async fn get_messages(
    pool: &SqlitePool,
    investigation_id: &str,
) -> Result<Vec<AgentMessageRow>> {
    query_as::<_, AgentMessageRow>(
        "SELECT * FROM agent_messages WHERE investigation_id = ?1 ORDER BY created_at ASC",
    )
    .bind(investigation_id)
    .fetch_all(pool)
    .await
    .context("cannot list messages")
}

pub async fn get_findings(pool: &SqlitePool, investigation_id: &str) -> Result<Vec<FindingRow>> {
    query_as::<_, FindingRow>(
        "SELECT * FROM findings WHERE investigation_id = ?1 ORDER BY created_at DESC",
    )
    .bind(investigation_id)
    .fetch_all(pool)
    .await
    .context("cannot list findings")
}

pub async fn get_sources(
    pool: &SqlitePool,
    investigation_id: &str,
) -> Result<Vec<SourceDocumentRow>> {
    query_as::<_, SourceDocumentRow>(
        "SELECT * FROM source_documents WHERE investigation_id = ?1 ORDER BY fetched_at DESC",
    )
    .bind(investigation_id)
    .fetch_all(pool)
    .await
    .context("cannot list sources")
}

pub async fn list_heartbeats(pool: &SqlitePool) -> Result<Vec<HeartbeatRow>> {
    query_as::<_, HeartbeatRow>("SELECT * FROM heartbeats ORDER BY last_seen_at DESC")
        .fetch_all(pool)
        .await
        .context("cannot list heartbeats")
}

pub async fn relevant_heartbeats(
    pool: &SqlitePool,
    investigation_id: &str,
) -> Result<Vec<HeartbeatRow>> {
    query_as::<_, HeartbeatRow>(
        r#"
        SELECT *
        FROM heartbeats
        WHERE component = 'service'
           OR component = 'agent'
           OR (component = 'investigation' AND scope = ?1)
        ORDER BY last_seen_at DESC
        "#,
    )
    .bind(investigation_id)
    .fetch_all(pool)
    .await
    .context("cannot list relevant heartbeats")
}

pub async fn upsert_heartbeat(
    pool: &SqlitePool,
    component: &str,
    scope: &str,
    status_text: &str,
    ttl_seconds: i64,
    details: Value,
) -> Result<()> {
    query(
        r#"
        INSERT INTO heartbeats (component, scope, status_text, last_seen_at, ttl_seconds, details_json)
        VALUES (?1, ?2, ?3, ?4, ?5, ?6)
        ON CONFLICT(component, scope) DO UPDATE SET
          status_text = excluded.status_text,
          last_seen_at = excluded.last_seen_at,
          ttl_seconds = excluded.ttl_seconds,
          details_json = excluded.details_json
        "#,
    )
    .bind(component)
    .bind(scope)
    .bind(status_text)
    .bind(now_rfc3339())
    .bind(ttl_seconds)
    .bind(to_json_string(&details))
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn due_schedules(pool: &SqlitePool) -> Result<Vec<ScheduleRow>> {
    query_as::<_, ScheduleRow>(
        r#"
        SELECT *
        FROM schedules
        WHERE enabled = 1
          AND next_run_at IS NOT NULL
          AND next_run_at <= ?1
        ORDER BY next_run_at ASC
        "#,
    )
    .bind(now_rfc3339())
    .fetch_all(pool)
    .await
    .context("cannot list due schedules")
}

pub async fn mark_schedule_executed(pool: &SqlitePool, schedule: &ScheduleRow) -> Result<()> {
    let now = now_rfc3339();
    let next_run_at = schedule_next_run(&schedule.cron_expr)?;
    query(
        r#"
        UPDATE schedules
        SET last_run_at = ?2,
            next_run_at = ?3,
            updated_at = ?2
        WHERE id = ?1
        "#,
    )
    .bind(&schedule.id)
    .bind(now)
    .bind(next_run_at)
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn list_schedules(pool: &SqlitePool) -> Result<Vec<ScheduleRow>> {
    query_as::<_, ScheduleRow>("SELECT * FROM schedules ORDER BY name ASC")
        .fetch_all(pool)
        .await
        .context("cannot list schedules")
}

pub async fn create_schedule(
    pool: &SqlitePool,
    request: &crate::models::CreateScheduleRequest,
) -> Result<ScheduleRow> {
    let id = new_id();
    let now = now_rfc3339();
    let next_run_at = schedule_next_run(&request.cron_expr)?;
    query(
        r#"
        INSERT INTO schedules (id, name, cron_expr, job_type, enabled, payload_json, last_run_at, next_run_at, updated_at)
        VALUES (?1, ?2, ?3, ?4, ?5, ?6, NULL, ?7, ?8)
        "#,
    )
    .bind(&id)
    .bind(&request.name)
    .bind(&request.cron_expr)
    .bind(&request.job_type)
    .bind(if request.enabled { 1 } else { 0 })
    .bind(to_json_string(&request.payload))
    .bind(next_run_at)
    .bind(now)
    .execute(pool)
    .await?;

    query_as::<_, ScheduleRow>("SELECT * FROM schedules WHERE id = ?1")
        .bind(id)
        .fetch_one(pool)
        .await
        .context("cannot fetch created schedule")
}

pub async fn update_schedule(
    pool: &SqlitePool,
    schedule_id: &str,
    request: &crate::models::UpdateScheduleRequest,
) -> Result<ScheduleRow> {
    let current = query_as::<_, ScheduleRow>("SELECT * FROM schedules WHERE id = ?1")
        .bind(schedule_id)
        .fetch_one(pool)
        .await
        .with_context(|| format!("schedule {schedule_id} not found"))?;

    let cron_expr = request
        .cron_expr
        .clone()
        .unwrap_or_else(|| current.cron_expr.clone());
    let enabled = request.enabled.unwrap_or(current.enabled == 1);
    let payload = request
        .payload
        .clone()
        .unwrap_or_else(|| from_json_value(&current.payload_json));
    let next_run_at = schedule_next_run(&cron_expr)?;

    query(
        r#"
        UPDATE schedules
        SET cron_expr = ?2,
            enabled = ?3,
            payload_json = ?4,
            next_run_at = ?5,
            updated_at = ?6
        WHERE id = ?1
        "#,
    )
    .bind(schedule_id)
    .bind(cron_expr)
    .bind(if enabled { 1 } else { 0 })
    .bind(to_json_string(&payload))
    .bind(next_run_at)
    .bind(now_rfc3339())
    .execute(pool)
    .await?;

    query_as::<_, ScheduleRow>("SELECT * FROM schedules WHERE id = ?1")
        .bind(schedule_id)
        .fetch_one(pool)
        .await
        .context("cannot fetch updated schedule")
}

pub async fn sweep_stale_heartbeats(pool: &SqlitePool) -> Result<usize> {
    let now = Utc::now();
    let mut swept = 0usize;
    for heartbeat in list_heartbeats(pool).await? {
        let last_seen = parse_rfc3339(&heartbeat.last_seen_at)?;
        if now.signed_duration_since(last_seen).num_seconds() > heartbeat.ttl_seconds {
            query(
                "UPDATE heartbeats SET status_text = 'stale' WHERE component = ?1 AND scope = ?2",
            )
            .bind(&heartbeat.component)
            .bind(&heartbeat.scope)
            .execute(pool)
            .await?;
            swept += 1;
        }
    }
    Ok(swept)
}

pub async fn compact_history(pool: &SqlitePool) -> Result<usize> {
    let threshold = (Utc::now() - Duration::days(30)).to_rfc3339_opts(SecondsFormat::Secs, true);
    let removed_runs =
        query("DELETE FROM agent_runs WHERE completed_at IS NOT NULL AND completed_at < ?1")
            .bind(&threshold)
            .execute(pool)
            .await?
            .rows_affected() as usize;

    let removed_heartbeats =
        query("DELETE FROM heartbeats WHERE component = 'investigation' AND last_seen_at < ?1")
            .bind(&threshold)
            .execute(pool)
            .await?
            .rows_affected() as usize;

    Ok(removed_runs + removed_heartbeats)
}

pub fn to_investigation_summary(row: InvestigationRow) -> InvestigationSummary {
    InvestigationSummary {
        id: row.id,
        topic: row.topic,
        goal: row.goal,
        status: row.status,
        source_scope: row.source_scope,
        priority: row.priority,
        summary: row.summary,
        final_report: row.final_report,
        tags: from_json_vec(&row.tags_json),
        seed_urls: from_json_vec(&row.seed_urls_json),
        created_at: row.created_at,
        updated_at: row.updated_at,
        completed_at: row.completed_at,
    }
}

pub fn to_section_view(row: SectionRow) -> SectionView {
    SectionView {
        id: row.id,
        slug: row.slug,
        title: row.title,
        status: row.status,
        conclusion: row.conclusion,
        position: row.position,
        updated_at: row.updated_at,
    }
}

pub fn to_message_view(row: AgentMessageRow) -> crate::models::MessageView {
    crate::models::MessageView {
        id: row.id,
        section_id: row.section_id,
        agent_role: row.agent_role,
        target_role: row.target_role,
        kind: row.kind,
        content: row.content,
        citations: from_json_vec(&row.citations_json),
        confidence: row.confidence,
        created_at: row.created_at,
    }
}

pub fn to_finding_view(row: FindingRow) -> FindingView {
    FindingView {
        id: row.id,
        section_id: row.section_id,
        agent_role: row.agent_role,
        kind: row.kind,
        title: row.title,
        summary: row.summary,
        direction: row.direction,
        confidence: row.confidence,
        evidence: from_json_vec(&row.evidence_json),
        created_at: row.created_at,
    }
}

pub fn to_source_view(row: SourceDocumentRow) -> SourceView {
    SourceView {
        id: row.id,
        url: row.url,
        title: row.title,
        fetched_at: row.fetched_at,
        excerpt: row.excerpt,
        metadata: from_json_value(&row.metadata_json),
    }
}

pub fn to_heartbeat_view(row: HeartbeatRow) -> HeartbeatView {
    let health = parse_rfc3339(&row.last_seen_at)
        .ok()
        .map(|last_seen| {
            let age_seconds = Utc::now().signed_duration_since(last_seen).num_seconds();
            if age_seconds <= row.ttl_seconds / 2 {
                "healthy"
            } else if age_seconds <= row.ttl_seconds {
                "delayed"
            } else {
                "stale"
            }
        })
        .unwrap_or("stale")
        .to_string();

    HeartbeatView {
        component: row.component,
        scope: row.scope,
        status_text: row.status_text,
        health,
        last_seen_at: row.last_seen_at,
        ttl_seconds: row.ttl_seconds,
        details: from_json_value(&row.details_json),
    }
}

pub fn to_schedule_view(row: ScheduleRow) -> ScheduleView {
    ScheduleView {
        id: row.id,
        name: row.name,
        cron_expr: row.cron_expr,
        job_type: row.job_type,
        enabled: row.enabled == 1,
        payload: from_json_value(&row.payload_json),
        last_run_at: row.last_run_at,
        next_run_at: row.next_run_at,
        updated_at: row.updated_at,
    }
}

pub async fn build_dashboard(pool: &SqlitePool) -> Result<DashboardResponse> {
    let day_ago = (Utc::now() - Duration::days(1)).to_rfc3339_opts(SecondsFormat::Secs, true);
    let stats = DashboardStats {
        total_investigations: query_scalar_count(pool, "SELECT COUNT(1) FROM investigations")
            .await?,
        running_investigations: query_scalar_count(
            pool,
            "SELECT COUNT(1) FROM investigations WHERE status = 'running'",
        )
        .await?,
        completed_investigations: query_scalar_count(
            pool,
            "SELECT COUNT(1) FROM investigations WHERE status = 'completed'",
        )
        .await?,
        recent_findings: query_as::<_, (i64,)>(
            "SELECT COUNT(1) FROM findings WHERE created_at >= ?1",
        )
        .bind(day_ago)
        .fetch_one(pool)
        .await
        .map(|row| row.0)
        .unwrap_or(0),
        stale_heartbeats: list_heartbeats(pool)
            .await?
            .into_iter()
            .map(to_heartbeat_view)
            .filter(|heartbeat| heartbeat.health == "stale")
            .count() as i64,
    };

    let recent_investigations = list_investigation_rows(pool)
        .await?
        .into_iter()
        .take(6)
        .map(to_investigation_summary)
        .collect();

    let recent_findings =
        query_as::<_, FindingRow>("SELECT * FROM findings ORDER BY created_at DESC LIMIT 8")
            .fetch_all(pool)
            .await?
            .into_iter()
            .map(to_finding_view)
            .collect();

    let schedules = list_schedules(pool)
        .await?
        .into_iter()
        .map(to_schedule_view)
        .collect();

    let heartbeats = list_heartbeats(pool)
        .await?
        .into_iter()
        .map(to_heartbeat_view)
        .collect();

    Ok(DashboardResponse {
        stats,
        recent_investigations,
        recent_findings,
        agent_statuses: build_agent_statuses(pool).await?,
        schedules,
        heartbeats,
    })
}

pub async fn build_agent_statuses(pool: &SqlitePool) -> Result<Vec<AgentStatusView>> {
    let heartbeats = list_heartbeats(pool).await?;
    let messages = query_as::<_, AgentMessageRow>(
        "SELECT * FROM agent_messages ORDER BY created_at DESC LIMIT 100",
    )
    .fetch_all(pool)
    .await?;

    let mut statuses = Vec::with_capacity(AgentRole::team().len());
    for role in AgentRole::team() {
        let heartbeat = heartbeats
            .iter()
            .find(|item| item.component == "agent" && item.scope == role.as_str());
        let last_message = messages
            .iter()
            .find(|item| item.agent_role == role.as_str())
            .map(|item| item.content.clone());
        let open_runs = query_as::<_, (i64,)>(
            "SELECT COUNT(1) FROM agent_runs WHERE agent_role = ?1 AND status = 'running'",
        )
        .bind(role.as_str())
        .fetch_one(pool)
        .await
        .map(|row| row.0)
        .unwrap_or(0);

        statuses.push(AgentStatusView {
            role: role.as_str().to_string(),
            label: role.label().to_string(),
            status: heartbeat
                .map(|item| item.status_text.clone())
                .unwrap_or_else(|| "idle".to_string()),
            last_seen_at: heartbeat.map(|item| item.last_seen_at.clone()),
            last_message,
            open_runs,
        });
    }

    Ok(statuses)
}

pub async fn build_investigation_detail(
    pool: &SqlitePool,
    investigation_id: &str,
) -> Result<InvestigationDetail> {
    let investigation =
        to_investigation_summary(get_investigation_row(pool, investigation_id).await?);
    let sections = get_sections(pool, investigation_id)
        .await?
        .into_iter()
        .map(to_section_view)
        .collect();
    let transcript = get_messages(pool, investigation_id)
        .await?
        .into_iter()
        .map(to_message_view)
        .collect();
    let findings = get_findings(pool, investigation_id)
        .await?
        .into_iter()
        .map(to_finding_view)
        .collect();
    let sources = get_sources(pool, investigation_id)
        .await?
        .into_iter()
        .map(to_source_view)
        .collect();
    let heartbeats = relevant_heartbeats(pool, investigation_id)
        .await?
        .into_iter()
        .map(to_heartbeat_view)
        .collect();

    Ok(InvestigationDetail {
        investigation,
        sections,
        transcript,
        findings,
        sources,
        heartbeats,
    })
}

async fn query_scalar_count(pool: &SqlitePool, sql: &str) -> Result<i64> {
    let row = query_as::<_, (i64,)>(sql).fetch_one(pool).await?;
    Ok(row.0)
}

#[cfg(test)]
mod tests {
    use chrono::{DateTime, Utc};

    use super::{schedule_next_run, slugify_section};

    #[test]
    fn computes_next_schedule_run() {
        let next = schedule_next_run("0 * * * * *").expect("schedule should parse");
        let parsed = DateTime::parse_from_rfc3339(&next).expect("next run should parse");
        assert!(parsed.with_timezone(&Utc) > Utc::now());
    }

    #[test]
    fn keeps_known_section_slugs_stable() {
        assert_eq!(slugify_section("Tin hieu ky thuat", 1), "technical_signals");
        assert_eq!(slugify_section("Mau thuan va rui ro", 2), "contradictions");
    }
}
