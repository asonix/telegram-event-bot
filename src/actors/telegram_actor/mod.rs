use actix::{Actor, Address, Context, Handler};
use failure::Fail;
use futures::Future;
use telebot::RcBot;
use telebot::functions::FunctionMessage;

use actors::db_actor::DbActor;
use actors::db_actor::messages::GetChatSystemByEventId;
use error::EventErrorKind;
use models::event::Event;

pub mod messages;

use self::messages::*;

pub struct TelegramActor {
    bot: RcBot,
    db: Address<DbActor>,
}

impl TelegramActor {
    fn notify_event(&self, event: Event) {
        let bot = self.bot.clone();

        let fut = self.db
            .call_fut(GetChatSystemByEventId {
                event_id: event.id(),
            })
            .then(|msg_res| match msg_res {
                Ok(res) => res,
                Err(err) => Err(err.context(EventErrorKind::Cancelled).into()),
            })
            .and_then(move |chat_system| {
                bot.message(
                    chat_system.events_channel(),
                    format!("Don't forget! {} is starting soon!", event.title()),
                ).send()
                    .map_err(|e| e.context(EventErrorKind::Telegram).into())
            })
            .map(|_| ())
            .map_err(|_| ());

        self.bot.inner.handle.spawn(fut);
    }
}

impl Actor for TelegramActor {
    type Context = Context<Self>;
}

impl Handler<NotifyEvent> for TelegramActor {
    type Result = ();

    fn handle(&mut self, msg: NotifyEvent, _: &mut Self::Context) -> Self::Result {
        self.notify_event(msg.0);
    }
}
