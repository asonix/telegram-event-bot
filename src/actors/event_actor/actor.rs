use actix::{Actor, Context, Handler, ResponseFuture};
use actix::fut::wrap_future;
use event_web::{EditEvent, LookupEvent, NewEvent};

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

impl Handler<LookupEvent> for EventActor {
    type Result = ResponseFuture<Self, LookupEvent>;

    fn handle(&mut self, msg: LookupEvent, _: &mut Self::Context) -> Self::Result {
        Box::new(wrap_future::<_, Self>(self.lookup_event(msg.0)))
    }
}

impl Handler<EditEvent> for EventActor {
    type Result = ();

    fn handle(&mut self, msg: EditEvent, _: &mut Self::Context) -> Self::Result {
        self.edit_event(msg.0, msg.1);
    }
}
