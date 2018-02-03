-- Your SQL goes here
CREATE TABLE events (
  id          SERIAL UNIQUE PRIMARY KEY,
  start_date  TIMESTAMP WITH TIME ZONE,
  end_date    TIMESTAMP WITH TIME ZONE,
  title       TEXT,
  description TEXT,
  system_id   INTEGER REFERENCES chat_systems ON DELETE CASCADE
);
