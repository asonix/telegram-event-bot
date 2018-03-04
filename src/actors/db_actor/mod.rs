use actix::Address;
use chrono::DateTime;
use chrono::offset::Utc;
use futures::{Future, IntoFuture};
use telebot::objects::Integer;
use tokio_postgres::Connection;

use actors::db_broker::DbBroker;
use models::event::{CreateEvent, Event};
use models::chat::{Chat, CreateChat};
use models::chat_system::ChatSystem;
use models::new_event_link::NewEventLink;
use models::user::{CreateUser, User};

use error::{EventError, EventErrorKind};

mod actor;
pub mod messages;

pub struct DbActor {
    broker: Address<DbBroker>,
    connection: Option<Connection>,
}

impl DbActor {
    pub fn new(broker: Address<DbBroker>, connection: Connection) -> Self {
        DbActor {
            broker: broker,
            connection: Some(connection),
        }
    }

    fn take_connection(&mut self) -> Result<Connection, EventError> {
        self.connection
            .take()
            .ok_or(EventErrorKind::MissingConnection.into())
    }

    fn insert_event(
        &mut self,
        system_id: i32,
        title: String,
        description: String,
        start_date: DateTime<Utc>,
        end_date: DateTime<Utc>,
        hosts: Vec<i32>,
    ) -> impl Future<Item = (Event, Connection), Error = Result<(EventError, Connection), EventError>>
    {
        self.take_connection()
            .into_future()
            .map_err(Err)
            .and_then(move |connection| ChatSystem::by_id(system_id, connection).map_err(Ok))
            .and_then(move |(chat_system, connection)| {
                User::by_ids(hosts, connection)
                    .map_err(Ok)
                    .map(|(hosts, connection)| (chat_system, hosts, connection))
            })
            .and_then(move |(chat_system, hosts, connection)| {
                let new_event = CreateEvent {
                    start_date,
                    end_date,
                    title,
                    description,
                    hosts,
                };

                new_event.create(&chat_system, connection).map_err(Ok)
            })
    }

    fn delete_event(
        &mut self,
        event_id: i32,
    ) -> impl Future<Item = ((), Connection), Error = Result<(EventError, Connection), EventError>>
    {
        self.take_connection()
            .into_future()
            .map_err(Err)
            .and_then(move |connection| Event::delete_by_id(event_id, connection).map_err(Ok))
            .and_then(|(count, connection)| {
                if count == 1 {
                    Ok(((), connection))
                } else {
                    Err(Ok((EventErrorKind::Delete.into(), connection)))
                }
            })
    }

    fn delete_chat_system(
        &mut self,
        channel_id: Integer,
    ) -> impl Future<Item = ((), Connection), Error = Result<(EventError, Connection), EventError>>
    {
        self.take_connection()
            .into_future()
            .map_err(Err)
            .and_then(move |connection| {
                ChatSystem::by_channel_id(channel_id, connection).map_err(Ok)
            })
            .and_then(move |(chat_system, connection)| chat_system.delete(connection).map_err(Ok))
            .and_then(|(count, connection)| {
                // TODO: move this to chat_system module
                if count == 1 {
                    Ok(((), connection))
                } else {
                    Err(Ok((EventErrorKind::Delete.into(), connection)))
                }
            })
    }

    fn insert_channel(
        &mut self,
        channel_id: Integer,
    ) -> impl Future<Item = (ChatSystem, Connection), Error = Result<(EventError, Connection), EventError>>
    {
        self.take_connection()
            .into_future()
            .map_err(Err)
            .and_then(move |connection| ChatSystem::create(channel_id, connection).map_err(Ok))
    }

    fn insert_chat(
        &mut self,
        channel_id: Integer,
        chat_id: Integer,
    ) -> impl Future<Item = (Chat, Connection), Error = Result<(EventError, Connection), EventError>>
    {
        self.take_connection()
            .into_future()
            .map_err(Err)
            .and_then(move |connection| {
                ChatSystem::by_channel_id(channel_id, connection).map_err(Ok)
            })
            .and_then(move |(chat_system, connection)| {
                let new_chat = CreateChat { chat_id };

                new_chat.create(&chat_system, connection).map_err(Ok)
            })
    }

    fn new_user(
        &mut self,
        chat_id: Integer,
        user_id: Integer,
    ) -> impl Future<Item = (User, Connection), Error = Result<(EventError, Connection), EventError>>
    {
        self.take_connection()
            .into_future()
            .map_err(Err)
            .and_then(move |connection| Chat::by_chat_id(chat_id, connection).map_err(Ok))
            .and_then(move |(chat, connection)| {
                let new_user = CreateUser { user_id };

                new_user.create(&chat, connection).map_err(Ok)
            })
    }

    fn new_user_chat_relation(
        &mut self,
        chat_id: Integer,
        user_id: Integer,
    ) -> impl Future<Item = ((), Connection), Error = Result<(EventError, Connection), EventError>>
    {
        self.take_connection()
            .into_future()
            .map_err(Err)
            .and_then(move |connection| {
                CreateUser::create_relation(user_id, chat_id, connection).map_err(Ok)
            })
    }

    fn get_events_in_range(
        &mut self,
        start_date: DateTime<Utc>,
        end_date: DateTime<Utc>,
    ) -> impl Future<Item = (Vec<Event>, Connection), Error = Result<(EventError, Connection), EventError>>
    {
        self.take_connection()
            .into_future()
            .map_err(Err)
            .and_then(move |connection| {
                Event::in_range(start_date, end_date, connection).map_err(Ok)
            })
    }

    fn get_events_for_system(
        &mut self,
        system_id: i32,
    ) -> impl Future<Item = (Vec<Event>, Connection), Error = Result<(EventError, Connection), EventError>>
    {
        self.take_connection()
            .into_future()
            .map_err(Err)
            .and_then(move |connection| Event::by_system_id(system_id, connection).map_err(Ok))
    }

    fn get_chat_system_by_event_id(
        &mut self,
        event_id: i32,
    ) -> impl Future<Item = (ChatSystem, Connection), Error = Result<(EventError, Connection), EventError>>
    {
        self.take_connection()
            .into_future()
            .map_err(Err)
            .and_then(move |connection| ChatSystem::by_event_id(event_id, connection).map_err(Ok))
    }

    fn get_system_by_id(
        &mut self,
        system_id: i32,
    ) -> impl Future<Item = (ChatSystem, Connection), Error = Result<(EventError, Connection), EventError>>
    {
        self.take_connection()
            .into_future()
            .map_err(Err)
            .and_then(move |connection| ChatSystem::by_id(system_id, connection).map_err(Ok))
    }

    fn get_system_by_channel(
        &mut self,
        channel_id: Integer,
    ) -> impl Future<Item = (ChatSystem, Connection), Error = Result<(EventError, Connection), EventError>>
    {
        self.take_connection()
            .into_future()
            .map_err(Err)
            .and_then(move |connection| {
                ChatSystem::by_channel_id(channel_id, connection).map_err(Ok)
            })
    }

    fn get_users_with_chats(
        &mut self,
    ) -> impl Future<
        Item = (Vec<(User, Chat)>, Connection),
        Error = Result<(EventError, Connection), EventError>,
    > {
        self.take_connection()
            .into_future()
            .map_err(Err)
            .and_then(move |connection| User::get_with_chats(connection).map_err(Ok))
    }

    fn store_event_link(
        &mut self,
        user_id: i32,
        system_id: i32,
        secret: String,
    ) -> impl Future<
        Item = (NewEventLink, Connection),
        Error = Result<(EventError, Connection), EventError>,
    > {
        self.take_connection()
            .into_future()
            .map_err(Err)
            .and_then(move |connection| {
                NewEventLink::create(user_id, system_id, secret, connection).map_err(Ok)
            })
    }

    fn get_event_link_by_user_id(
        &mut self,
        user_id: i32,
    ) -> impl Future<
        Item = (NewEventLink, Connection),
        Error = Result<(EventError, Connection), EventError>,
    > {
        self.take_connection()
            .into_future()
            .map_err(Err)
            .and_then(move |connection| NewEventLink::by_user_id(user_id, connection).map_err(Ok))
    }

    fn delete_event_link(
        &mut self,
        id: i32,
    ) -> impl Future<Item = ((), Connection), Error = Result<(EventError, Connection), EventError>>
    {
        self.take_connection()
            .into_future()
            .map_err(Err)
            .and_then(move |connection| NewEventLink::delete(id, connection).map_err(Ok))
            .map(|c| ((), c))
    }

    fn lookup_user(
        &mut self,
        user_id: Integer,
    ) -> impl Future<Item = (User, Connection), Error = Result<(EventError, Connection), EventError>>
    {
        self.take_connection()
            .into_future()
            .map_err(Err)
            .and_then(move |connection| {
                User::by_user_ids(vec![user_id], connection)
                    .and_then(|(mut users, connection)| {
                        if users.len() > 0 {
                            Ok((users.remove(0), connection))
                        } else {
                            Err((EventErrorKind::Lookup.into(), connection))
                        }
                    })
                    .map_err(Ok)
            })
    }

    fn get_systems_with_chats(
        &mut self,
    ) -> impl Future<
        Item = (Vec<(ChatSystem, Chat)>, Connection),
        Error = Result<(EventError, Connection), EventError>,
    > {
        self.take_connection()
            .into_future()
            .map_err(Err)
            .and_then(move |connection| ChatSystem::all_with_chats(connection).map_err(Ok))
    }

    fn remove_user_chat(
        &mut self,
        user_id: Integer,
        chat_id: Integer,
    ) -> impl Future<Item = ((), Connection), Error = Result<(EventError, Connection), EventError>>
    {
        debug!(
            "Deleting relation between chat {} and user {}",
            chat_id, user_id
        );
        self.take_connection()
            .into_future()
            .map_err(Err)
            .and_then(move |connection| {
                User::delete_relation_by_ids(user_id, chat_id, connection).map_err(Ok)
            })
    }

    fn delete_user_by_user_id(
        &mut self,
        user_id: Integer,
    ) -> impl Future<Item = ((), Connection), Error = Result<(EventError, Connection), EventError>>
    {
        self.take_connection()
            .into_future()
            .map_err(Err)
            .and_then(move |connection| User::delete_by_user_id(user_id, connection).map_err(Ok))
    }
}
