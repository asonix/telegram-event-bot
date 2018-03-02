use actix::Address;
use event_web::Event as FrontendEvent;
use futures::Future;
use telebot::objects::Integer;

use actors::db_broker::DbBroker;
use actors::telegram_actor::TelegramActor;
use error::EventError;

pub struct EventActor {
    tg: Address<TelegramActor>,
    db: Address<DbBroker>,
}

impl EventActor {
    pub fn new(tg: Address<TelegramActor>, db: Address<DbBroker>) -> Self {
        EventActor { tg, db }
    }

    fn new_event(&mut self, _event: FrontendEvent, _id: String) {
        /*
        Arbiter::handle().spawn(
            self.db.
        )
        */
        ()
    }
}

mod actor {
    use actix::{Actor, Context, Handler};
    use event_web::NewEvent;

    use super::EventActor;

    impl Actor for EventActor {
        type Context = Context<Self>;
    }

    impl Handler<NewEvent> for EventActor {
        type Result = ();

        fn handle(&mut self, msg: NewEvent, _: &mut Self::Context) -> Self::Result {
            self.new_event(msg.0, msg.1);
        }
    }
}
