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
    fn new_event(&mut self, _event: FrontendEvent, _id: String) {
        /*
        Arbiter::handle().spawn(
            self.db.
        )
        */
        ()
    }

    fn request_new_event(
        &self,
        _user_id: Integer,
        _chat_id: Integer,
        _channel_id: Integer,
    ) -> Box<Future<Item = String, Error = EventError>> {
        use futures::future::result;

        Box::new(result(Ok("Hey".to_owned())))
        // self.db.call_fut(RequestNewEvent(user_id, chat_id, channel_id))
    }
}

mod actor {
    use actix::{Actor, Context, Handler, ResponseFuture};
    use actix::fut::wrap_future;
    use event_web::NewEvent;

    use super::EventActor;
    use super::messages::*;

    impl Actor for EventActor {
        type Context = Context<Self>;
    }

    impl Handler<NewEvent> for EventActor {
        type Result = ();

        fn handle(&mut self, msg: NewEvent, _: &mut Self::Context) -> Self::Result {
            self.new_event(msg.0, msg.1);
        }
    }

    impl Handler<RequestNewEvent> for EventActor {
        type Result = ResponseFuture<Self, RequestNewEvent>;

        fn handle(&mut self, msg: RequestNewEvent, _: &mut Self::Context) -> Self::Result {
            Box::new(wrap_future(self.request_new_event(
                msg.user_id,
                msg.chat_id,
                msg.channel_id,
            )))
        }
    }
}

pub mod messages {
    use actix::ResponseType;
    use telebot::objects::Integer;

    use error::EventError;

    #[derive(Clone, Debug, Eq, Hash, PartialEq)]
    pub struct RequestNewEvent {
        pub user_id: Integer,
        pub chat_id: Integer,
        pub channel_id: Integer,
    }

    impl ResponseType for RequestNewEvent {
        type Item = String;
        type Error = EventError;
    }
}
