-- Your SQL goes here
CREATE TABLE new_event_links (
    id        SERIAL UNIQUE PRIMARY KEY,
    users_id  INTEGER REFERENCES users ON DELETE CASCADE,
    system_id INTEGER REFERENCES chat_systems ON DELETE CASCADE,
    used      BOOLEAN DEFAULT FALSE,
    secret    TEXT UNIQUE
);
