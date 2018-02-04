use futures::Future;
use futures_state_stream::StateStream;
use telebot::objects::Integer;
use tokio_postgres::Connection;

use chat::Chat;
use chat_system::ChatSystem;
use error::{EventError, EventErrorKind};
use util::*;

/// User represents a user that belongs to at least one chat in a system
///
/// `user_id` is the user's ID
///
/// ### Relations:
/// - users has_many user_chats (foreign key on user_chats)
///
/// ### Columns:
/// - id SERIAL
/// - user_id BIGINT
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct User {
    id: i32,
    user_id: Integer,
}

impl User {
    pub fn from_parts(id: i32, user_id: Integer) -> Self {
        User { id, user_id }
    }

    pub fn maybe_from_parts(id: Option<i32>, user_id: Option<Integer>) -> Option<Self> {
        id.and_then(|id| user_id.map(|user_id| User { id, user_id }))
    }

    pub fn id(&self) -> i32 {
        self.id
    }

    pub fn user_id(&self) -> Integer {
        self.user_id
    }

    pub fn get_systems(
        &self,
        connection: Connection,
    ) -> Box<Future<Item = (Vec<ChatSystem>, Connection), Error = (EventError, Connection)>> {
        ChatSystem::by_user_id(self.user_id, connection)
    }

    pub fn delete(
        self,
        connection: Connection,
    ) -> Box<Future<Item = (u64, Connection), Error = (EventError, Connection)>> {
        let sql = "DELETE FROM users AS usr WHERE usr.id = $1";

        Box::new(
            connection
                .prepare(sql)
                .map_err(prepare_error)
                .and_then(move |(s, connection)| {
                    connection.execute(&s, &[&self.id]).map_err(delete_error)
                }),
        )
    }
}

pub struct CreateUser {
    pub user_id: Integer,
}

impl CreateUser {
    pub fn create(
        self,
        chat: &Chat,
        connection: Connection,
    ) -> Box<Future<Item = (User, Connection), Error = (EventError, Connection)>> {
        let sql = "INSERT INTO users (user_id) VALUES ($1) RETURNING id";
        let join_sql = "INSERT INTO user_chats (users_id, chats_id) VALUES ($1, $2)";

        let CreateUser { user_id } = self;

        let chats_id = chat.id();

        Box::new(
            connection
                .transaction()
                .map_err(transaction_error)
                .and_then(move |transaction| {
                    transaction
                        .prepare(sql)
                        .map_err(transaction_prepare_error)
                        .and_then(move |(s, transaction)| {
                            transaction
                                .query(&s, &[&user_id])
                                .map(move |row| User {
                                    id: row.get(0),
                                    user_id: user_id,
                                })
                                .collect()
                                .map_err(transaction_insert_error)
                        })
                        .and_then(|(mut users, transaction)| {
                            if users.len() == 1 {
                                Ok((users.remove(0), transaction))
                            } else {
                                Err((EventErrorKind::Insert.into(), transaction))
                            }
                        })
                        .and_then(move |(user, transaction)| {
                            let users_id = user.id();
                            transaction
                                .prepare(join_sql)
                                .map_err(transaction_prepare_error)
                                .and_then(move |(s, transaction)| {
                                    transaction
                                        .execute(&s, &[&users_id, &chats_id])
                                        .map_err(transaction_insert_error)
                                        .and_then(|(count, transaction)| {
                                            if count == 1 {
                                                Ok((user, transaction))
                                            } else {
                                                Err((EventErrorKind::Insert.into(), transaction))
                                            }
                                        })
                                })
                        })
                        .or_else(|(error, transaction)| {
                            transaction
                                .rollback()
                                .or_else(|(_, connection)| Err(connection))
                                .then(move |res| match res {
                                    Ok(connection) => Err((error, connection)),
                                    Err(connection) => Err((error, connection)),
                                })
                        })
                })
                .and_then(|(user, transaction)| {
                    transaction
                        .commit()
                        .map_err(commit_error)
                        .map(move |connection| (user, connection))
                }),
        )
    }
}
