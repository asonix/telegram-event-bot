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
