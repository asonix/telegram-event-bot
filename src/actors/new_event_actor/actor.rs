use actix::{Actor, Address, AsyncContext, Context, Handler, ResponseFuture};
use actix::fut::wrap_future;

use error::EventError;
use super::messages::*;
use super::NewEventActor;

impl Actor for NewEventActor {
    type Context = Context<Self>;
}

impl Handler<NewTitle> for NewEventActor {
    type Result = ();

    fn handle(&mut self, msg: NewTitle, _: &mut Self::Context) -> Self::Result {
        self.new_title(msg.user_id, msg.title, msg.channel_id, msg.chat_id)
    }
}

impl Handler<AddDescription> for NewEventActor {
    type Result = Result<(), EventError>;

    fn handle(&mut self, msg: AddDescription, _: &mut Self::Context) -> Self::Result {
        self.add_description(msg.user_id, msg.description, msg.chat_id)
    }
}

impl Handler<AddDate> for NewEventActor {
    type Result = Result<(), EventError>;

    fn handle(&mut self, msg: AddDate, _: &mut Self::Context) -> Self::Result {
        self.add_date(msg.user_id, msg.start_date, msg.chat_id)
    }
}

impl Handler<AddEnd> for NewEventActor {
    type Result = Result<(), EventError>;

    fn handle(&mut self, msg: AddEnd, _: &mut Self::Context) -> Self::Result {
        self.add_end(msg.user_id, msg.end_date, msg.chat_id)
    }
}

impl Handler<AddHost> for NewEventActor {
    type Result = Result<(), EventError>;

    fn handle(&mut self, msg: AddHost, _: &mut Self::Context) -> Self::Result {
        self.add_host(msg.user_id, msg.host, msg.chat_id)
    }
}

impl Handler<Finalize> for NewEventActor {
    type Result = ResponseFuture<Self, Finalize>;

    fn handle(&mut self, msg: Finalize, _: &mut Self::Context) -> Self::Result {
        Box::new(wrap_future::<_, Self>(self.finalize(
            msg.user_id,
            msg.chat_id,
        )))
    }
}

impl Handler<Incoming> for NewEventActor {
    type Result = ();

    fn handle(&mut self, msg: Incoming, ctx: &mut Self::Context) -> Self::Result {
        let address: Address<_> = ctx.address();

        if self.titles.contains_key(&msg.user_id) {
            address.send(AddDescription {
                user_id: msg.user_id,
                description: msg.message,
                chat_id: msg.chat_id,
            });
        } else {
            address.send(NewTitle {
                channel_id: msg.channel_id,
                user_id: msg.user_id,
                title: msg.message,
                chat_id: msg.chat_id,
            })
        }
    }
}
