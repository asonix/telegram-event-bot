use actix::{Actor, Context, Handler};

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

impl Handler<UpdateEvent> for TelegramActor {
    type Result = ();

    fn handle(&mut self, msg: UpdateEvent, _: &mut Self::Context) -> Self::Result {
        self.update_event(msg.0);
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
