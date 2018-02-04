-- This file should undo anything in `up.sql`
ALTER TABLE hosts
DROP COLUMN users_id;

ALTER TABLE hosts
ADD COLUMN user_id BIGINT;
