-- Your SQL goes here
CREATE TABLE hosts (
  id        SERIAL UNIQUE PRIMARY KEY,
  user_id   BIGINT,
  events_id INTEGER REFERENCES events ON DELETE CASCADE
);
