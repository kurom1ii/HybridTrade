CREATE TABLE IF NOT EXISTS instruments (
  symbol TEXT PRIMARY KEY,
  name TEXT NOT NULL DEFAULT '',
  category TEXT NOT NULL DEFAULT 'forex',
  direction TEXT NOT NULL DEFAULT 'neutral',
  confidence REAL NOT NULL DEFAULT 0.0,
  price REAL NOT NULL DEFAULT 0.0,
  change_pct REAL NOT NULL DEFAULT 0.0,
  timeframe TEXT NOT NULL DEFAULT '',
  analysis TEXT NOT NULL DEFAULT '',
  key_levels TEXT NOT NULL DEFAULT '[]',
  updated_at TEXT NOT NULL
);
