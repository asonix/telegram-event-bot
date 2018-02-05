use actix::{ActorFuture, Handler, ResponseFuture};
use actix::fut::wrap_future;

use super::DbActor;
use super::messages::*;

impl Handler<NewChannel> for DbActor {
    type Result = ResponseFuture<Self, NewChannel>;

    fn handle(&mut self, msg: NewChannel, _: &mut Self::Context) -> Self::Result {
        let NewChannel { channel_id } = msg;

        Box::new(
            wrap_future::<_, Self>(self.insert_channel(channel_id))
                .map(|(chat_system, connection), db_actor, _| {
                    db_actor.connection = Some(connection);

                    chat_system
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

impl Handler<DeleteChannel> for DbActor {
    type Result = ResponseFuture<Self, DeleteChannel>;

    fn handle(&mut self, msg: DeleteChannel, _: &mut Self::Context) -> Self::Result {
        let DeleteChannel { channel_id } = msg;

        Box::new(
            wrap_future::<_, Self>(self.delete_chat_system(channel_id))
                .map(|connection, db_actor, _| {
                    db_actor.connection = Some(connection);

                    ()
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

impl Handler<NewChat> for DbActor {
    type Result = ResponseFuture<Self, NewChat>;

    fn handle(&mut self, msg: NewChat, _: &mut Self::Context) -> Self::Result {
        let NewChat {
            channel_id,
            chat_id,
        } = msg;

        Box::new(
            wrap_future::<_, Self>(self.insert_chat(channel_id, chat_id))
                .map(|(chat, connection), db_actor, _| {
                    db_actor.connection = Some(connection);

                    chat
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

impl Handler<NewUser> for DbActor {
    type Result = ResponseFuture<Self, NewUser>;

    fn handle(&mut self, msg: NewUser, _: &mut Self::Context) -> Self::Result {
        let NewUser { chat_id, user_id } = msg;

        Box::new(
            wrap_future::<_, Self>(self.insert_user(chat_id, user_id))
                .map(|(user, connection), db_actor, _| {
                    db_actor.connection = Some(connection);

                    user
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

impl Handler<NewEvent> for DbActor {
    type Result = ResponseFuture<Self, NewEvent>;

    fn handle(&mut self, msg: NewEvent, _: &mut Self::Context) -> Self::Result {
        let NewEvent {
            channel_id,
            title,
            description,
            start_date,
            end_date,
            hosts,
        } = msg;

        Box::new(
            wrap_future::<_, Self>(self.insert_event(
                channel_id,
                title,
                description,
                start_date,
                end_date,
                hosts,
            )).map(|(event, connection), db_actor, _| {
                db_actor.connection = Some(connection);

                event
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
