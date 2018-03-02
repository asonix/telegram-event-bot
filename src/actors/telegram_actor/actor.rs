use actix::{Actor, Context, Handler};

use super::messages::*;
use super::TelegramActor;

impl Actor for TelegramActor {
    type Context = Context<Self>;
}

impl Handler<NotifyEvent> for TelegramActor {
    type Result = ();

    fn handle(&mut self, msg: NotifyEvent, _: &mut Self::Context) -> Self::Result {
        self.notify_event(msg.0);
    }
}

impl Handler<EventOver> for TelegramActor {
    type Result = ();

    fn handle(&mut self, msg: EventOver, _: &mut Self::Context) -> Self::Result {
        self.query_events(msg.event_id, msg.system_id);
    }
}

impl Handler<AskChats> for TelegramActor {
    type Result = ();

    fn handle(&mut self, msg: AskChats, _: &mut Self::Context) -> Self::Result {
        self.ask_chats(msg.0, msg.1)
    }
}
