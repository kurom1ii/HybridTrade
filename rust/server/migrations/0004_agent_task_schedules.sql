-- Drop heartbeats table (no longer needed)
DROP TABLE IF EXISTS heartbeats;

-- Add agent task fields to schedules
ALTER TABLE schedules ADD COLUMN agent_role TEXT NOT NULL DEFAULT 'kuromi';
ALTER TABLE schedules ADD COLUMN message TEXT NOT NULL DEFAULT '';
ALTER TABLE schedules ADD COLUMN last_status TEXT NOT NULL DEFAULT 'idle';
ALTER TABLE schedules ADD COLUMN last_result TEXT;
