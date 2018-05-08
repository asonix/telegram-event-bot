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

//! This module defines the DbBroker, a struct that manages access to database conections

use std::cell::RefCell;
use std::collections::VecDeque;
use std::rc::Rc;

use chrono::DateTime;
use chrono_tz::Tz;
use futures::task;
use futures::{Async, Future, Poll};
use telebot::objects::Integer;
use tokio_postgres::Connection;

use error::{EventError, EventErrorKind};
use models::chat::{Chat, CreateChat};
use models::chat_system::ChatSystem;
use models::edit_event_link::EditEventLink;
use models::event::{CreateEvent, Event, UpdateEvent};
use models::new_event_link::NewEventLink;
use models::user::{CreateUser, User};

mod actor;
pub mod messages;

/// Define the structure that contains the `Connection` collection
///
/// This wraps an Rc<RefCell<>> to allow multiple future chains on the DbBroker to have access to
/// the connections.
///
/// Future is implemented for this type, and since it can be easily cloned, multiple futures can
/// wait on the presence of a `Connection` in the pool.
pub struct Connections(Rc<RefCell<VecDeque<Connection>>>);

impl Clone for Connections {
    fn clone(&self) -> Self {
        Connections(Rc::clone(&self.0))
    }
}

impl Default for Connections {
    fn default() -> Self {
        Connections(Rc::new(RefCell::new(VecDeque::default())))
    }
}

impl Future for Connections {
    type Item = Connection;
    type Error = EventError;

    fn poll(&mut self) -> Poll<Self::Item, Self::Error> {
        if let Some(item) = self.0.borrow_mut().pop_front() {
            Ok(Async::Ready(item))
        } else {
            // busy wait until we have a connection to use
            task::current().notify();
            Ok(Async::NotReady)
        }
    }
}

/// Define the DbBroker. This struct manages access to the connections, and additionally contains
/// the database url to ensure that new connections can be created.
pub struct DbBroker {
    num_connections: usize,
    db_url: String,
    connections: Connections,
}

impl DbBroker {
    pub fn new(db_url: String, num_connections: usize) -> Self {
        DbBroker {
            num_connections: num_connections,
            db_url: db_url,
            connections: Connections::default(),
        }
    }

    fn insert_event(
        system_id: i32,
        title: String,
        description: String,
        start_date: DateTime<Tz>,
        end_date: DateTime<Tz>,
        hosts: Vec<i32>,
        connection: Connection,
    ) -> impl Future<Item = (Event, Connection), Error = (EventError, Connection)> {
        User::by_ids(hosts, connection)
            .map(|(hosts, connection)| (hosts, connection))
            .and_then(move |(hosts, connection)| {
                let new_event = CreateEvent {
                    system_id,
                    start_date,
                    end_date,
                    title,
                    description,
                    hosts,
                };

                new_event.create(connection)
            })
    }

    fn edit_event(
        id: i32,
        system_id: i32,
        title: String,
        description: String,
        start_date: DateTime<Tz>,
        end_date: DateTime<Tz>,
        hosts: Vec<i32>,
        connection: Connection,
    ) -> impl Future<Item = (Event, Connection), Error = (EventError, Connection)> {
        let updated_event = UpdateEvent {
            id,
            system_id,
            start_date,
            end_date,
            title,
            description,
            hosts,
        };

        updated_event.update(connection)
    }

    fn lookup_event(
        event_id: i32,
        connection: Connection,
    ) -> impl Future<Item = (Event, Connection), Error = (EventError, Connection)> {
        Event::by_id(event_id, connection)
    }

    fn lookup_events_by_user_id(
        user_id: Integer,
        connection: Connection,
    ) -> impl Future<Item = (Vec<Event>, Connection), Error = (EventError, Connection)> {
        Event::by_user_id(user_id, connection)
    }

    fn delete_event(
        event_id: i32,
        connection: Connection,
    ) -> impl Future<Item = ((), Connection), Error = (EventError, Connection)> {
        Event::delete_by_id(event_id, connection).and_then(|(count, connection)| {
            if count == 1 {
                Ok(((), connection))
            } else {
                Err((EventErrorKind::Delete.into(), connection))
            }
        })
    }

    fn delete_chat_system(
        channel_id: Integer,
        connection: Connection,
    ) -> impl Future<Item = ((), Connection), Error = (EventError, Connection)> {
        ChatSystem::by_channel_id(channel_id, connection)
            .and_then(move |(chat_system, connection)| chat_system.delete(connection))
            .and_then(|(count, connection)| {
                // TODO: move this to chat_system module
                if count == 1 {
                    Ok(((), connection))
                } else {
                    Err((EventErrorKind::Delete.into(), connection))
                }
            })
    }

    fn insert_channel(
        channel_id: Integer,
        connection: Connection,
    ) -> impl Future<Item = (ChatSystem, Connection), Error = (EventError, Connection)> {
        ChatSystem::create(channel_id, connection)
    }

    fn insert_chat(
        channel_id: Integer,
        chat_id: Integer,
        connection: Connection,
    ) -> impl Future<Item = (Chat, Connection), Error = (EventError, Connection)> {
        ChatSystem::by_channel_id(channel_id, connection).and_then(
            move |(chat_system, connection)| {
                let new_chat = CreateChat { chat_id };

                new_chat.create(&chat_system, connection)
            },
        )
    }

    fn new_user(
        chat_id: Integer,
        user_id: Integer,
        username: String,
        connection: Connection,
    ) -> impl Future<Item = (User, Connection), Error = (EventError, Connection)> {
        Chat::by_chat_id(chat_id, connection).and_then(move |(chat, connection)| {
            let new_user = CreateUser { user_id, username };

            new_user.create(&chat, connection)
        })
    }

    fn new_user_chat_relation(
        chat_id: Integer,
        user_id: Integer,
        connection: Connection,
    ) -> impl Future<Item = ((), Connection), Error = (EventError, Connection)> {
        CreateUser::create_relation(user_id, chat_id, connection)
    }

    fn get_events_by_chat_id(
        chat_id: Integer,
        connection: Connection,
    ) -> impl Future<Item = (Vec<Event>, Connection), Error = (EventError, Connection)> {
        Event::by_chat_id(chat_id, connection)
    }

    fn get_events_in_range(
        start_date: DateTime<Tz>,
        end_date: DateTime<Tz>,
        connection: Connection,
    ) -> impl Future<Item = (Vec<Event>, Connection), Error = (EventError, Connection)> {
        Event::in_range(start_date, end_date, connection)
    }

    fn get_events_for_system(
        system_id: i32,
        connection: Connection,
    ) -> impl Future<Item = (Vec<Event>, Connection), Error = (EventError, Connection)> {
        Event::by_system_id(system_id, connection)
    }

    fn get_system_by_id(
        system_id: i32,
        connection: Connection,
    ) -> impl Future<Item = (ChatSystem, Connection), Error = (EventError, Connection)> {
        ChatSystem::by_id(system_id, connection)
    }

    fn get_system_by_channel(
        channel_id: Integer,
        connection: Connection,
    ) -> impl Future<Item = (ChatSystem, Connection), Error = (EventError, Connection)> {
        ChatSystem::by_channel_id(channel_id, connection)
    }

    fn get_users_with_chats(
        connection: Connection,
    ) -> impl Future<Item = (Vec<(User, Chat)>, Connection), Error = (EventError, Connection)> {
        User::get_with_chats(connection)
    }

    fn store_edit_event_link(
        user_id: i32,
        system_id: i32,
        event_id: i32,
        secret: String,
        connection: Connection,
    ) -> impl Future<Item = (EditEventLink, Connection), Error = (EventError, Connection)> {
        EditEventLink::create(user_id, system_id, event_id, secret, connection)
    }

    fn get_edit_event_link(
        id: i32,
        connection: Connection,
    ) -> impl Future<Item = (EditEventLink, Connection), Error = (EventError, Connection)> {
        EditEventLink::by_id(id, connection)
    }

    fn delete_edit_event_link(
        id: i32,
        connection: Connection,
    ) -> impl Future<Item = ((), Connection), Error = (EventError, Connection)> {
        EditEventLink::delete(id, connection).map(|c| ((), c))
    }

    fn store_event_link(
        user_id: i32,
        system_id: i32,
        secret: String,
        connection: Connection,
    ) -> impl Future<Item = (NewEventLink, Connection), Error = (EventError, Connection)> {
        NewEventLink::create(user_id, system_id, secret, connection)
    }

    fn get_event_link(
        id: i32,
        connection: Connection,
    ) -> impl Future<Item = (NewEventLink, Connection), Error = (EventError, Connection)> {
        NewEventLink::by_id(id, connection)
    }

    fn delete_event_link(
        id: i32,
        connection: Connection,
    ) -> impl Future<Item = ((), Connection), Error = (EventError, Connection)> {
        NewEventLink::delete(id, connection).map(|c| ((), c))
    }

    fn lookup_user(
        user_id: Integer,
        connection: Connection,
    ) -> impl Future<Item = (User, Connection), Error = (EventError, Connection)> {
        User::by_user_ids(vec![user_id], connection).and_then(|(mut users, connection)| {
            if users.len() > 0 {
                Ok((users.remove(0), connection))
            } else {
                Err((EventErrorKind::Lookup.into(), connection))
            }
        })
    }

    fn get_systems_with_chats(
        connection: Connection,
    ) -> impl Future<Item = (Vec<(ChatSystem, Chat)>, Connection), Error = (EventError, Connection)>
    {
        ChatSystem::all_with_chats(connection)
    }

    fn remove_user_chat(
        user_id: Integer,
        chat_id: Integer,
        connection: Connection,
    ) -> impl Future<Item = ((), Connection), Error = (EventError, Connection)> {
        debug!(
            "Deleting relation between chat {} and user {}",
            chat_id, user_id
        );
        User::delete_relation_by_ids(user_id, chat_id, connection)
    }

    fn delete_user_by_user_id(
        user_id: Integer,
        connection: Connection,
    ) -> impl Future<Item = ((), Connection), Error = (EventError, Connection)> {
        User::delete_by_user_id(user_id, connection)
    }
}
