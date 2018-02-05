use actix::{Actor, Context};
use futures::Future;
use telebot::objects::Integer;
use tokio_postgres::Connection;

use models::user::{CreateUser, User};
use models::chat::{Chat, CreateChat};
use models::chat_system::ChatSystem;

use error::{EventError, EventErrorKind};

mod actor;
pub mod messages;

pub struct DbActor {
    connection: Option<Connection>,
}

impl Actor for DbActor {
    type Context = Context<Self>;
}

impl DbActor {
    fn take_connection(&mut self) -> Result<Connection, EventError> {
        self.connection
            .take()
            .ok_or(EventErrorKind::MissingConnection.into())
    }

    fn insert_channel(
        &mut self,
        channel_id: Integer,
    ) -> Box<
        Future<
            Item = (ChatSystem, Connection),
            Error = Result<(EventError, Connection), EventError>,
        >,
    > {
        use futures::{Future, IntoFuture};

        Box::new(
            self.take_connection()
                .into_future()
                .map_err(Err)
                .and_then(move |connection| ChatSystem::create(channel_id, connection).map_err(Ok)),
        )
    }

    fn insert_chat(
        &mut self,
        channel_id: Integer,
        chat_id: Integer,
    ) -> Box<Future<Item = (Chat, Connection), Error = Result<(EventError, Connection), EventError>>>
    {
        use futures::{Future, IntoFuture};

        Box::new(
            self.take_connection()
                .into_future()
                .map_err(Err)
                .and_then(move |connection| {
                    ChatSystem::by_channel_id(channel_id, connection).map_err(Ok)
                })
                .and_then(move |(chat_system, connection)| {
                    let new_chat = CreateChat { chat_id };

                    new_chat.create(&chat_system, connection).map_err(Ok)
                }),
        )
    }

    fn insert_user(
        &mut self,
        chat_id: Integer,
        user_id: Integer,
    ) -> Box<Future<Item = (User, Connection), Error = Result<(EventError, Connection), EventError>>>
    {
        use futures::{Future, IntoFuture};

        Box::new(
            self.take_connection()
                .into_future()
                .map_err(Err)
                .and_then(move |connection| Chat::by_chat_id(chat_id, connection).map_err(Ok))
                .and_then(move |(chat, connection)| {
                    let new_user = CreateUser { user_id };

                    new_user.create(&chat, connection).map_err(Ok)
                }),
        )
    }
}
