-- Your SQL goes here
CREATE TABLE user_chats (
    id          SERIAL UNIQUE PRIMARY KEY,
    users_id    INTEGER REFERENCES users ON DELETE CASCADE,
    chats_id    INTEGER REFERENCES chats ON DELETE CASCADE
);
