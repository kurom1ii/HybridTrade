-- Per-schedule tool / MCP / skill filtering
ALTER TABLE schedules ADD COLUMN allowed_tools TEXT;
ALTER TABLE schedules ADD COLUMN allowed_mcps  TEXT;
ALTER TABLE schedules ADD COLUMN skills        TEXT;
