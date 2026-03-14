use anyhow::{anyhow, Context, Result};
use chrono::{DateTime, SecondsFormat, Utc};
use cron::Schedule;
use serde::Serialize;
use serde_json::Value;
use sqlx::{query, query_as, SqlitePool};

use crate::{
    agents::AgentRole,
    config::ScheduleSeed,
    models::{
        new_id, AgentStatusView, DashboardResponse, ScheduleRow, ScheduleView,
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

pub fn from_json_value(value: &str) -> Value {
    serde_json::from_str(value).unwrap_or(Value::Null)
}

/// Normalize a cron expression: the `cron` crate requires 6 or 7 fields
/// (seconds minutes hours dom month dow [year]).  If the user supplies a
/// standard 5-field expression we prepend "0" for the seconds field.
fn normalize_cron(expr: &str) -> String {
    let fields = expr.split_whitespace().count();
    if fields == 5 {
        format!("0 {expr}")
    } else {
        expr.to_string()
    }
}

pub fn schedule_next_run(cron_expr: &str) -> Result<String> {
    let normalized = normalize_cron(cron_expr);
    let schedule = normalized.parse::<Schedule>()?;
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
            INSERT INTO schedules (id, name, cron_expr, job_type, enabled, payload_json, last_run_at, next_run_at, updated_at, agent_role, message)
            VALUES (?1, ?2, ?3, ?4, ?5, ?6, NULL, ?7, ?8, ?9, ?10)
            ON CONFLICT(name) DO UPDATE SET
              cron_expr = excluded.cron_expr,
              job_type = excluded.job_type,
              enabled = excluded.enabled,
              payload_json = excluded.payload_json,
              next_run_at = excluded.next_run_at,
              updated_at = excluded.updated_at,
              agent_role = excluded.agent_role,
              message = excluded.message
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
        .bind(&schedule.agent_role)
        .bind(&schedule.message)
        .execute(pool)
        .await?;
    }

    Ok(())
}

pub async fn schedule_exists(pool: &SqlitePool, schedule_id: &str) -> Result<bool> {
    let row = query_as::<_, (i64,)>("SELECT COUNT(1) FROM schedules WHERE id = ?1")
        .bind(schedule_id)
        .fetch_one(pool)
        .await?;
    Ok(row.0 > 0)
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

pub async fn update_schedule_status(
    pool: &SqlitePool,
    id: &str,
    status: &str,
    result: Option<&str>,
) -> Result<()> {
    query(
        r#"
        UPDATE schedules
        SET last_status = ?2,
            last_result = ?3,
            updated_at = ?4
        WHERE id = ?1
        "#,
    )
    .bind(id)
    .bind(status)
    .bind(result)
    .bind(now_rfc3339())
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
        INSERT INTO schedules (id, name, cron_expr, job_type, enabled, payload_json, last_run_at, next_run_at, updated_at, agent_role, message)
        VALUES (?1, ?2, ?3, ?4, ?5, ?6, NULL, ?7, ?8, ?9, ?10)
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
    .bind(&request.agent_role)
    .bind(&request.message)
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
        agent_role: row.agent_role,
        message: row.message,
        last_status: row.last_status,
        last_result: row.last_result,
    }
}

pub async fn build_dashboard(pool: &SqlitePool) -> Result<DashboardResponse> {
    let schedules = list_schedules(pool)
        .await?
        .into_iter()
        .map(to_schedule_view)
        .collect();

    Ok(DashboardResponse {
        agent_statuses: build_agent_statuses(pool).await?,
        schedules,
    })
}

pub async fn build_agent_statuses(pool: &SqlitePool) -> Result<Vec<AgentStatusView>> {
    let schedules = list_schedules(pool).await?;

    let mut statuses = Vec::with_capacity(AgentRole::visible_agents().len());
    for role in AgentRole::visible_agents() {
        let role_schedules: Vec<&ScheduleRow> = schedules
            .iter()
            .filter(|s| role.matches_stored_role(&s.agent_role))
            .collect();

        let running_count = role_schedules
            .iter()
            .filter(|s| s.last_status == "running")
            .count() as i64;

        let status = if running_count > 0 {
            "running".to_string()
        } else if role_schedules.iter().any(|s| s.enabled == 1) {
            "idle".to_string()
        } else {
            "idle".to_string()
        };

        let last_run = role_schedules
            .iter()
            .filter_map(|s| s.last_run_at.as_ref())
            .max()
            .cloned();

        let last_result = role_schedules
            .iter()
            .filter_map(|s| s.last_result.as_ref())
            .last()
            .cloned();

        statuses.push(AgentStatusView {
            role: role.as_str().to_string(),
            label: role.label().to_string(),
            status,
            last_seen_at: last_run,
            last_message: last_result,
            open_runs: running_count,
        });
    }

    Ok(statuses)
}

#[cfg(test)]
mod tests {
    use chrono::{DateTime, Utc};

    use super::{normalize_cron, schedule_next_run};

    #[test]
    fn computes_next_schedule_run() {
        let next = schedule_next_run("0 * * * * *").expect("schedule should parse");
        let parsed = DateTime::parse_from_rfc3339(&next).expect("next run should parse");
        assert!(parsed.with_timezone(&Utc) > Utc::now());
    }

    #[test]
    fn five_field_cron_works() {
        // Standard 5-field cron like `*/5 * * * *` should be auto-normalized
        let next = schedule_next_run("*/5 * * * *").expect("5-field cron should parse");
        let parsed = DateTime::parse_from_rfc3339(&next).expect("next run should parse");
        assert!(parsed.with_timezone(&Utc) > Utc::now());
    }

    #[test]
    fn normalize_cron_prepends_seconds() {
        assert_eq!(normalize_cron("*/5 * * * *"), "0 */5 * * * *");
        assert_eq!(normalize_cron("0 9 * * 1-5"), "0 0 9 * * 1-5");
        // Already 6 fields — no change
        assert_eq!(normalize_cron("0 */5 * * * *"), "0 */5 * * * *");
    }
}
