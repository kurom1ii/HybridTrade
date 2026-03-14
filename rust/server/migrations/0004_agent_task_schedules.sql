-- Upgrade schedules table: add agent task fields
-- These columns enable the scheduler to dispatch tasks to agents

-- Drop legacy heartbeats table
DROP TABLE IF EXISTS heartbeats;

-- Recreate schedules with new columns (safe upgrade for SQLite)
-- SQLite doesn't support ADD COLUMN IF NOT EXISTS, so we use a
-- temporary table approach only if the columns are missing.
-- Since sqlx tracks migrations, this only runs on DBs created
-- before these columns were added to 0001_init.sql.

CREATE TABLE IF NOT EXISTS schedules_new (
  id TEXT PRIMARY KEY,
  name TEXT NOT NULL UNIQUE,
  cron_expr TEXT NOT NULL,
  job_type TEXT NOT NULL DEFAULT 'agent_task',
  enabled INTEGER NOT NULL DEFAULT 1,
  payload_json TEXT NOT NULL DEFAULT '{}',
  last_run_at TEXT,
  next_run_at TEXT,
  updated_at TEXT NOT NULL,
  agent_role TEXT NOT NULL DEFAULT 'kuromi',
  message TEXT NOT NULL DEFAULT '',
  last_status TEXT NOT NULL DEFAULT 'idle',
  last_result TEXT
);

INSERT OR IGNORE INTO schedules_new (id, name, cron_expr, job_type, enabled, payload_json, last_run_at, next_run_at, updated_at)
  SELECT id, name, cron_expr, job_type, enabled, payload_json, last_run_at, next_run_at, updated_at
  FROM schedules;

DROP TABLE schedules;
ALTER TABLE schedules_new RENAME TO schedules;
