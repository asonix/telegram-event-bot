-- Your SQL goes here
CREATE TABLE chat_systems (
  id              SERIAL UNIQUE PRIMARY KEY,
  events_channel  BIGINT UNIQUE
);
