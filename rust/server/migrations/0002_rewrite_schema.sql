PRAGMA foreign_keys = ON;

DROP TRIGGER IF EXISTS memory_items_ai;
DROP TRIGGER IF EXISTS memory_items_ad;
DROP TRIGGER IF EXISTS memory_items_au;

DROP TABLE IF EXISTS checkpoints;
DROP TABLE IF EXISTS tool_invocations;
DROP TABLE IF EXISTS memory_items_fts;
DROP TABLE IF EXISTS memory_items;
DROP TABLE IF EXISTS source_documents;
DROP TABLE IF EXISTS findings;
DROP TABLE IF EXISTS agent_messages;
DROP TABLE IF EXISTS agent_runs;
DROP TABLE IF EXISTS investigation_sections;
DROP TABLE IF EXISTS investigations;
DROP TABLE IF EXISTS schedules;
DROP TABLE IF EXISTS heartbeats;

CREATE TABLE investigations (
  id TEXT PRIMARY KEY,
  topic TEXT NOT NULL,
  goal TEXT NOT NULL,
  status TEXT NOT NULL,
  source_scope TEXT NOT NULL,
  priority TEXT NOT NULL,
  summary TEXT,
  final_report TEXT,
  tags_json TEXT NOT NULL DEFAULT '[]',
  seed_urls_json TEXT NOT NULL DEFAULT '[]',
  created_at TEXT NOT NULL,
  updated_at TEXT NOT NULL,
  completed_at TEXT
);

CREATE TABLE investigation_sections (
  id TEXT PRIMARY KEY,
  investigation_id TEXT NOT NULL REFERENCES investigations(id) ON DELETE CASCADE,
  slug TEXT NOT NULL,
  title TEXT NOT NULL,
  status TEXT NOT NULL,
  conclusion TEXT,
  position INTEGER NOT NULL,
  updated_at TEXT NOT NULL
);

CREATE INDEX idx_investigation_sections_investigation_id
  ON investigation_sections(investigation_id, position);

CREATE TABLE agent_runs (
  id TEXT PRIMARY KEY,
  investigation_id TEXT NOT NULL REFERENCES investigations(id) ON DELETE CASCADE,
  agent_role TEXT NOT NULL,
  status TEXT NOT NULL,
  started_at TEXT NOT NULL,
  completed_at TEXT
);

CREATE INDEX idx_agent_runs_investigation_id
  ON agent_runs(investigation_id, started_at DESC);

CREATE TABLE agent_messages (
  id TEXT PRIMARY KEY,
  investigation_id TEXT NOT NULL REFERENCES investigations(id) ON DELETE CASCADE,
  section_id TEXT REFERENCES investigation_sections(id) ON DELETE SET NULL,
  agent_role TEXT NOT NULL,
  target_role TEXT,
  kind TEXT NOT NULL,
  content TEXT NOT NULL,
  citations_json TEXT NOT NULL DEFAULT '[]',
  confidence REAL,
  created_at TEXT NOT NULL
);

CREATE INDEX idx_agent_messages_investigation_id
  ON agent_messages(investigation_id, created_at ASC);

CREATE TABLE findings (
  id TEXT PRIMARY KEY,
  investigation_id TEXT NOT NULL REFERENCES investigations(id) ON DELETE CASCADE,
  section_id TEXT REFERENCES investigation_sections(id) ON DELETE SET NULL,
  agent_role TEXT NOT NULL,
  kind TEXT NOT NULL,
  title TEXT NOT NULL,
  summary TEXT NOT NULL,
  direction TEXT,
  confidence REAL NOT NULL,
  evidence_json TEXT NOT NULL DEFAULT '[]',
  created_at TEXT NOT NULL
);

CREATE INDEX idx_findings_investigation_id
  ON findings(investigation_id, created_at DESC);

CREATE TABLE source_documents (
  id TEXT PRIMARY KEY,
  investigation_id TEXT NOT NULL REFERENCES investigations(id) ON DELETE CASCADE,
  url TEXT NOT NULL,
  title TEXT NOT NULL,
  fetched_at TEXT NOT NULL,
  excerpt TEXT,
  metadata_json TEXT NOT NULL DEFAULT '{}'
);

CREATE INDEX idx_source_documents_investigation_id
  ON source_documents(investigation_id, fetched_at DESC);

CREATE TABLE heartbeats (
  component TEXT NOT NULL,
  scope TEXT NOT NULL,
  status_text TEXT NOT NULL,
  last_seen_at TEXT NOT NULL,
  ttl_seconds INTEGER NOT NULL,
  details_json TEXT NOT NULL DEFAULT '{}',
  PRIMARY KEY(component, scope)
);

CREATE TABLE schedules (
  id TEXT PRIMARY KEY,
  name TEXT NOT NULL UNIQUE,
  cron_expr TEXT NOT NULL,
  job_type TEXT NOT NULL,
  enabled INTEGER NOT NULL DEFAULT 1,
  payload_json TEXT NOT NULL DEFAULT '{}',
  last_run_at TEXT,
  next_run_at TEXT,
  updated_at TEXT NOT NULL
);
