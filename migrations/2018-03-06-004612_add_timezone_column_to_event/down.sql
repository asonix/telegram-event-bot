-- This file should undo anything in `up.sql`
ALTER TABLE events
DROP COLUMN timezone;
