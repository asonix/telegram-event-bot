use actix::{Actor, Context, Handler, ResponseFuture};
use actix::fut::wrap_future;

use super::messages::*;
use super::TelegramActor;

impl Actor for TelegramActor {
    type Context = Context<Self>;
}

impl Handler<NewEvent> for TelegramActor {
    type Result = ();

    fn handle(&mut self, msg: NewEvent, _: &mut Self::Context) -> Self::Result {
        self.new_event(msg.0);
    }
}

impl Handler<EventSoon> for TelegramActor {
    type Result = ();

    fn handle(&mut self, msg: EventSoon, _: &mut Self::Context) -> Self::Result {
        self.event_soon(msg.0);
    }
}

impl Handler<EventStarted> for TelegramActor {
    type Result = ();

    fn handle(&mut self, msg: EventStarted, _: &mut Self::Context) -> Self::Result {
        self.event_started(msg.0);
    }
}

impl Handler<EventOver> for TelegramActor {
    type Result = ();

    fn handle(&mut self, msg: EventOver, _: &mut Self::Context) -> Self::Result {
        self.event_over(msg.0);
    }
}

impl Handler<AskChats> for TelegramActor {
    type Result = ();

    fn handle(&mut self, msg: AskChats, _: &mut Self::Context) -> Self::Result {
        self.ask_chats(msg.0, msg.1)
    }
}

impl Handler<IsAdmin> for TelegramActor {
    type Result = ResponseFuture<Self, IsAdmin>;

    fn handle(&mut self, msg: IsAdmin, _: &mut Self::Context) -> Self::Result {
        Box::new(wrap_future(self.is_admin(msg.0, msg.1)))
    }
}

impl Handler<Linked> for TelegramActor {
    type Result = ();

    fn handle(&mut self, msg: Linked, _: &mut Self::Context) -> Self::Result {
        self.linked(msg.0, msg.1)
    }
}

impl Handler<PrintId> for TelegramActor {
    type Result = ();

    fn handle(&mut self, msg: PrintId, _: &mut Self::Context) -> Self::Result {
        self.print_id(msg.0)
    }
}

impl Handler<CreatedChannel> for TelegramActor {
    type Result = ();

    fn handle(&mut self, msg: CreatedChannel, _: &mut Self::Context) -> Self::Result {
        self.created_channel(msg.0)
    }
}

impl Handler<SendUrl> for TelegramActor {
    type Result = ();

    fn handle(&mut self, msg: SendUrl, _: &mut Self::Context) -> Self::Result {
        self.send_url(msg.0, msg.1)
    }
}
