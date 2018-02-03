-- Your SQL goes here
CREATE TABLE chats (
  id        SERIAL UNIQUE PRIMARY KEY,
  chat_id   BIGINT,
  system_id INTEGER REFERENCES chat_systems ON DELETE CASCADE
);
