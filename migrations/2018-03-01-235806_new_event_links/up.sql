-- Your SQL goes here
CREATE TABLE new_event_links (
    id        SERIAL UNIQUE PRIMARY KEY,
    users_id  INTEGER UNIQUE REFERENCES users ON DELETE CASCADE,
    secret    TEXT UNIQUE
);
