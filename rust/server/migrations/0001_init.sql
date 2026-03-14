PRAGMA foreign_keys = ON;

CREATE TABLE IF NOT EXISTS schedules (
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
