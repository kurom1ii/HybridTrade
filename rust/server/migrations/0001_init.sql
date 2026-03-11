PRAGMA foreign_keys = ON;

CREATE TABLE IF NOT EXISTS investigations (
  id TEXT PRIMARY KEY,
  topic TEXT NOT NULL,
  goal TEXT NOT NULL,
  source_scope TEXT NOT NULL,
  priority TEXT NOT NULL,
  status TEXT NOT NULL,
  tags_json TEXT NOT NULL DEFAULT '[]',
  seed_urls_json TEXT NOT NULL DEFAULT '[]',
  summary TEXT,
  final_report TEXT,
  created_at TEXT NOT NULL,
  updated_at TEXT NOT NULL,
  completed_at TEXT
);

CREATE TABLE IF NOT EXISTS investigation_sections (
  id TEXT PRIMARY KEY,
  investigation_id TEXT NOT NULL REFERENCES investigations(id) ON DELETE CASCADE,
  slug TEXT NOT NULL,
  title TEXT NOT NULL,
  status TEXT NOT NULL,
  conclusion TEXT,
  position INTEGER NOT NULL,
  updated_at TEXT NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_investigation_sections_investigation_id ON investigation_sections(investigation_id, position);

CREATE TABLE IF NOT EXISTS agent_runs (
  id TEXT PRIMARY KEY,
  investigation_id TEXT NOT NULL REFERENCES investigations(id) ON DELETE CASCADE,
  agent_role TEXT NOT NULL,
  status TEXT NOT NULL,
  started_at TEXT NOT NULL,
  completed_at TEXT,
  error TEXT
);

CREATE INDEX IF NOT EXISTS idx_agent_runs_investigation_id ON agent_runs(investigation_id, started_at DESC);

CREATE TABLE IF NOT EXISTS agent_messages (
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

CREATE INDEX IF NOT EXISTS idx_agent_messages_investigation_id ON agent_messages(investigation_id, created_at ASC);

CREATE TABLE IF NOT EXISTS findings (
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

CREATE INDEX IF NOT EXISTS idx_findings_investigation_id ON findings(investigation_id, created_at DESC);

CREATE TABLE IF NOT EXISTS source_documents (
  id TEXT PRIMARY KEY,
  investigation_id TEXT NOT NULL REFERENCES investigations(id) ON DELETE CASCADE,
  url TEXT NOT NULL,
  title TEXT NOT NULL,
  fetched_at TEXT NOT NULL,
  content_hash TEXT NOT NULL,
  source_fingerprint TEXT NOT NULL,
  excerpt TEXT,
  raw_text TEXT NOT NULL,
  metadata_json TEXT NOT NULL DEFAULT '{}'
);

CREATE INDEX IF NOT EXISTS idx_source_documents_investigation_id ON source_documents(investigation_id, fetched_at DESC);
CREATE INDEX IF NOT EXISTS idx_source_documents_fingerprint ON source_documents(source_fingerprint);

CREATE TABLE IF NOT EXISTS memory_items (
  id TEXT PRIMARY KEY,
  investigation_id TEXT REFERENCES investigations(id) ON DELETE SET NULL,
  item_type TEXT NOT NULL,
  content TEXT NOT NULL,
  tags_json TEXT NOT NULL DEFAULT '[]',
  confidence REAL NOT NULL,
  created_at TEXT NOT NULL
);

CREATE VIRTUAL TABLE IF NOT EXISTS memory_items_fts USING fts5(
  content,
  tags,
  content='memory_items',
  content_rowid='rowid'
);

CREATE TRIGGER IF NOT EXISTS memory_items_ai AFTER INSERT ON memory_items BEGIN
  INSERT INTO memory_items_fts(rowid, content, tags)
  VALUES (new.rowid, new.content, new.tags_json);
END;

CREATE TRIGGER IF NOT EXISTS memory_items_ad AFTER DELETE ON memory_items BEGIN
  INSERT INTO memory_items_fts(memory_items_fts, rowid, content, tags)
  VALUES('delete', old.rowid, old.content, old.tags_json);
END;

CREATE TRIGGER IF NOT EXISTS memory_items_au AFTER UPDATE ON memory_items BEGIN
  INSERT INTO memory_items_fts(memory_items_fts, rowid, content, tags)
  VALUES('delete', old.rowid, old.content, old.tags_json);
  INSERT INTO memory_items_fts(rowid, content, tags)
  VALUES (new.rowid, new.content, new.tags_json);
END;

CREATE TABLE IF NOT EXISTS tool_invocations (
  id TEXT PRIMARY KEY,
  investigation_id TEXT REFERENCES investigations(id) ON DELETE SET NULL,
  agent_role TEXT NOT NULL,
  tool_name TEXT NOT NULL,
  tool_kind TEXT NOT NULL,
  status TEXT NOT NULL,
  request_json TEXT NOT NULL,
  response_json TEXT,
  error TEXT,
  created_at TEXT NOT NULL,
  completed_at TEXT
);

CREATE INDEX IF NOT EXISTS idx_tool_invocations_investigation_id ON tool_invocations(investigation_id, created_at DESC);

CREATE TABLE IF NOT EXISTS schedules (
  id TEXT PRIMARY KEY,
  name TEXT NOT NULL UNIQUE,
  cron_expr TEXT NOT NULL,
  job_type TEXT NOT NULL,
  payload_json TEXT NOT NULL DEFAULT '{}',
  enabled INTEGER NOT NULL DEFAULT 1,
  last_run_at TEXT,
  next_run_at TEXT,
  updated_at TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS heartbeats (
  component TEXT NOT NULL,
  scope TEXT NOT NULL,
  status_text TEXT NOT NULL,
  last_seen_at TEXT NOT NULL,
  ttl_seconds INTEGER NOT NULL,
  details_json TEXT NOT NULL DEFAULT '{}',
  PRIMARY KEY(component, scope)
);

CREATE TABLE IF NOT EXISTS checkpoints (
  id TEXT PRIMARY KEY,
  investigation_id TEXT REFERENCES investigations(id) ON DELETE CASCADE,
  checkpoint_type TEXT NOT NULL,
  payload_json TEXT NOT NULL,
  created_at TEXT NOT NULL
);

