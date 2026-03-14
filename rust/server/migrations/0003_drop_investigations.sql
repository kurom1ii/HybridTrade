-- Drop all investigation-related tables and their dependent tables
PRAGMA foreign_keys = ON;

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
