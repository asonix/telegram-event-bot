/*
 * This file is part of Telegram Event Bot.
 *
 * Copyright © 2018 Riley Trautman
 *
 * Telegram Event Bot is free software: you can redistribute it and/or modify
 * it under the terms of the GNU General Public License as published by
 * the Free Software Foundation, either version 3 of the License, or
 * (at your option) any later version.
 *
 * Telegram Event Bot is distributed in the hope that it will be useful,
 * but WITHOUT ANY WARRANTY; without even the implied warranty of
 * MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
 * GNU General Public License for more details.
 *
 * You should have received a copy of the GNU General Public License
 * along with Telegram Event Bot.  If not, see <http://www.gnu.org/licenses/>.
 */

//! This module defines the `User` struct and associated types and functions.

use futures::Future;
use futures_state_stream::StateStream;
use telebot::objects::Integer;
use tokio_postgres::Connection;
use tokio_postgres::types::ToSql;

use error::{EventError, EventErrorKind};
use super::chat::Chat;
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
/// - username TEXT
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct User {
    id: i32,
    user_id: Integer,
    username: String,
}

impl User {
    /// Construct a User from a series of Option types
    pub fn maybe_from_parts(
        id: Option<i32>,
        user_id: Option<Integer>,
        username: Option<String>,
    ) -> Option<Self> {
        Some(User {
            id: id?,
            user_id: user_id?,
            username: username?,
        })
    }

    /// Get the user's ID
    pub fn id(&self) -> i32 {
        self.id
    }

    /// Get the user's Telegram ID
    pub fn user_id(&self) -> Integer {
        self.user_id
    }

    /// Get the user's Telegram username
    pub fn username(&self) -> &str {
        &self.username
    }

    /// Get a `Vec<User>` given a list of Telegram IDs
    pub fn by_user_ids(
        user_ids: Vec<Integer>,
        connection: Connection,
    ) -> impl Future<Item = (Vec<User>, Connection), Error = (EventError, Connection)> {
        let sql = "SELECT usr.id, usr.user_id, usr.username FROM users AS usr WHERE usr.user_id IN";

        let values = user_ids
            .iter()
            .fold((Vec::new(), 1), |(mut acc, count), _| {
                acc.push(format!("${}", count));

                (acc, count + 1)
            })
            .0
            .join(", ");

        let full_sql = format!("{} ({})", sql, values);
        debug!("{}", full_sql);

        connection
            .prepare(&full_sql)
            .map_err(prepare_error)
            .and_then(move |(s, connection)| {
                let sql_args: Vec<_> = user_ids.iter().map(|user_id| user_id as &ToSql).collect();
                connection
                    .query(&s, sql_args.as_slice())
                    .map(move |row| User {
                        id: row.get(0),
                        user_id: row.get(1),
                        username: row.get(2),
                    })
                    .collect()
                    .map_err(lookup_error)
            })
    }

    /// Get a `Vec<User>` given a list of database IDs
    pub fn by_ids(
        ids: Vec<i32>,
        connection: Connection,
    ) -> impl Future<Item = (Vec<User>, Connection), Error = (EventError, Connection)> {
        let sql = "SELECT usr.id, usr.user_id, usr.username FROM users AS usr WHERE usr.id IN";

        let values = ids.iter()
            .fold((Vec::new(), 1), |(mut acc, count), _| {
                acc.push(format!("${}", count));

                (acc, count + 1)
            })
            .0
            .join(", ");

        let full_sql = format!("{} ({})", sql, values);
        debug!("{}", full_sql);

        connection
            .prepare(&full_sql)
            .map_err(prepare_error)
            .and_then(move |(s, connection)| {
                let sql_args: Vec<_> = ids.iter().map(|id| id as &ToSql).collect();
                connection
                    .query(&s, sql_args.as_slice())
                    .map(move |row| User {
                        id: row.get(0),
                        user_id: row.get(1),
                        username: row.get(2),
                    })
                    .collect()
                    .map_err(lookup_error)
            })
    }

    /// Get a vector of Users and their associated Chats
    pub fn get_with_chats(
        connection: Connection,
    ) -> impl Future<Item = (Vec<(User, Chat)>, Connection), Error = (EventError, Connection)> {
        let sql = "SELECT usr.id, usr.user_id, usr.username, ch.id, ch.chat_id
                    FROM users AS usr
                    INNER JOIN user_chats AS uc ON uc.users_id = usr.id
                    INNER JOIN chats AS ch ON uc.chats_id = ch.id";
        debug!("{}", sql);

        connection
            .prepare(sql)
            .map_err(prepare_error)
            .and_then(move |(s, connection)| {
                connection
                    .query(&s, &[])
                    .map(move |row| {
                        (
                            User {
                                id: row.get(0),
                                user_id: row.get(1),
                                username: row.get(2),
                            },
                            Chat::from_parts(row.get(3), row.get(4)),
                        )
                    })
                    .collect()
                    .map_err(lookup_error)
            })
    }

    /// Delete a User from the database
    pub fn delete_by_user_id(
        user_id: Integer,
        connection: Connection,
    ) -> impl Future<Item = ((), Connection), Error = (EventError, Connection)> {
        let sql = "DELETE FROM users AS usr WHERE usr.user_id = $1";
        debug!("{}", sql);

        connection
            .prepare(sql)
            .map_err(prepare_error)
            .and_then(move |(s, connection)| {
                connection.execute(&s, &[&user_id]).map_err(delete_error)
            })
            .and_then(|(count, connection)| {
                if count > 0 {
                    Ok(((), connection))
                } else {
                    Err((EventErrorKind::Delete.into(), connection))
                }
            })
    }

    /// Remove a relationship between a User and a Chat
    pub fn delete_relation_by_ids(
        user_id: Integer,
        chat_id: Integer,
        connection: Connection,
    ) -> impl Future<Item = ((), Connection), Error = (EventError, Connection)> {
        let sql = "DELETE FROM user_chats AS uc
                    USING users AS usr, chats AS ch
                    WHERE uc.users_id = usr.id AND uc.chats_id = ch.id AND usr.user_id = $1 AND ch.chat_id = $2";
        debug!("{}", sql);

        connection
            .prepare(sql)
            .map_err(prepare_error)
            .and_then(move |(s, connection)| {
                connection
                    .execute(&s, &[&user_id, &chat_id])
                    .map_err(delete_error)
                    .and_then(|(count, connection)| {
                        if count > 0 {
                            Ok(((), connection))
                        } else {
                            Err((EventErrorKind::Delete.into(), connection))
                        }
                    })
            })
    }
}

/// This type allows for safe insertion of Users into the database
pub struct CreateUser {
    pub user_id: Integer,
    pub username: String,
}

impl CreateUser {
    /// Create a relationship between the user with the given Telegram ID and the chat with the
    /// given Telegram ID
    pub fn create_relation(
        users_id: Integer,
        chats_id: Integer,
        connection: Connection,
    ) -> impl Future<Item = ((), Connection), Error = (EventError, Connection)> {
        let join_sql = "INSERT INTO user_chats (users_id, chats_id) VALUES ($1, $2)";

        connection
            .prepare(join_sql)
            .map_err(prepare_error)
            .and_then(move |(s, connection)| {
                connection
                    .execute(&s, &[&users_id, &chats_id])
                    .map_err(insert_error)
                    .and_then(|(count, connection)| {
                        if count == 1 {
                            Ok(((), connection))
                        } else {
                            Err((EventErrorKind::Insert.into(), connection))
                        }
                    })
            })
    }

    /// Create a User with the provided information
    pub fn create(
        self,
        chat: &Chat,
        connection: Connection,
    ) -> impl Future<Item = (User, Connection), Error = (EventError, Connection)> {
        let sql = "INSERT INTO users (user_id, username) VALUES ($1, $2) RETURNING id";
        let join_sql = "INSERT INTO user_chats (users_id, chats_id) VALUES ($1, $2)";

        let CreateUser { user_id, username } = self;

        let chats_id = chat.id();

        connection
            .transaction()
            .map_err(transaction_error)
            .and_then(move |transaction| {
                debug!("{}", sql);
                transaction
                    .prepare(sql)
                    .map_err(transaction_prepare_error)
                    .and_then(move |(s, transaction)| {
                        transaction
                            .query(&s, &[&user_id, &username])
                            .map(move |row| User {
                                id: row.get(0),
                                user_id: user_id,
                                username: username.clone(),
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
                        debug!("{}", join_sql);
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
            })
    }
}
