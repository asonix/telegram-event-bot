use actix::{Actor, ActorFuture, Context, Handler, ResponseFuture, ResponseType};
use actix::fut::{result, wrap_future};
use futures;
use telebot::objects::Integer;
use tokio_postgres::Connection;

use models::user::{CreateUser, User};
use models::chat::Chat;

use error::EventError;

pub struct DbActor {
    connection: Option<Connection>,
}

impl Actor for DbActor {
    type Context = Context<Self>;
}

impl Handler<NewUser> for DbActor {
    type Result = ResponseFuture<Self, NewUser>;

    fn handle(&mut self, msg: NewUser, ctx: &mut Self::Context) -> Self::Result {
        let connection = self.connection.take().unwrap();

        let NewUser { chat_id, user_id } = msg;

        Box::new(
            wrap_future::<_, Self>({
                futures::Future::and_then(
                    Chat::by_chat_id(msg.chat_id, connection),
                    move |(chat, connection)| {
                        let new_user = CreateUser { user_id: user_id };

                        new_user.create(&chat, connection)
                    },
                )
            }).map(|(user, connection), db_actor, _| {
                db_actor.connection = Some(connection);

                user
            })
                .map_err(|(error, connection), db_actor, _| {
                    db_actor.connection = Some(connection);

                    error
                }),
        )
    }
}

pub struct NewUser {
    chat_id: Integer,
    user_id: Integer,
}

impl ResponseType for NewUser {
    type Item = User;
    type Error = EventError;
}
