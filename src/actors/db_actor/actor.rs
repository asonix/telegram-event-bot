use actix::{ActorFuture, Handler, ResponseFuture};
use futures;
use tokio_postgres::Connection;

use error::EventError;
use super::DbActor;
use super::messages::*;

impl Handler<NewChannel> for DbActor {
    type Result = ResponseFuture<Self, NewChannel>;

    fn handle(&mut self, msg: NewChannel, _: &mut Self::Context) -> Self::Result {
        DbActor::wrap_fut(self.insert_channel(msg.channel_id))
    }
}

impl Handler<DeleteChannel> for DbActor {
    type Result = ResponseFuture<Self, DeleteChannel>;

    fn handle(&mut self, msg: DeleteChannel, _: &mut Self::Context) -> Self::Result {
        DbActor::wrap_fut(self.delete_chat_system(msg.channel_id))
    }
}

impl Handler<NewChat> for DbActor {
    type Result = ResponseFuture<Self, NewChat>;

    fn handle(&mut self, msg: NewChat, _: &mut Self::Context) -> Self::Result {
        DbActor::wrap_fut(self.insert_chat(msg.channel_id, msg.chat_id))
    }
}

impl Handler<NewUser> for DbActor {
    type Result = ResponseFuture<Self, NewUser>;

    fn handle(&mut self, msg: NewUser, _: &mut Self::Context) -> Self::Result {
        DbActor::wrap_fut(self.insert_user(msg.chat_id, msg.user_id))
    }
}

impl Handler<NewEvent> for DbActor {
    type Result = ResponseFuture<Self, NewEvent>;

    fn handle(&mut self, msg: NewEvent, _: &mut Self::Context) -> Self::Result {
        DbActor::wrap_fut(self.insert_event(
            msg.channel_id,
            msg.title,
            msg.description,
            msg.start_date,
            msg.end_date,
            msg.hosts,
        ))
    }
}

impl Handler<DeleteEvent> for DbActor {
    type Result = ResponseFuture<Self, DeleteEvent>;

    fn handle(&mut self, msg: DeleteEvent, _: &mut Self::Context) -> Self::Result {
        DbActor::wrap_fut(self.delete_event(msg.event_id))
    }
}

impl DbActor {
    fn wrap_fut<I, F>(fut: F) -> Box<ActorFuture<Item = I, Error = EventError, Actor = Self>>
    where
        F: futures::Future<
            Item = (I, Connection),
            Error = Result<(EventError, Connection), EventError>,
        >
            + 'static,
    {
        use actix::fut::wrap_future;

        Box::new(
            wrap_future::<_, Self>(fut)
                .map(|(item, connection), db_actor, _| {
                    db_actor.connection = Some(connection);

                    item
                })
                .map_err(|res, db_actor, _| match res {
                    Ok((error, connection)) => {
                        db_actor.connection = Some(connection);

                        error
                    }
                    Err(error) => error,
                }),
        )
    }
}
