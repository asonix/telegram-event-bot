extern crate actix;
extern crate event_web;

use actix::{Actor, Context, Handler, System};
use event_web::{EditEvent, Event, FrontendError, FrontendErrorKind, LookupEvent, NewEvent};

#[derive(Copy, Clone, Debug)]
struct MyHandler;

impl Actor for MyHandler {
    type Context = Context<Self>;
}

impl Handler<NewEvent> for MyHandler {
    type Result = ();

    fn handle(&mut self, msg: NewEvent, _: &mut Self::Context) -> Self::Result {
        println!("Event: {:?}", msg.0);
    }
}

impl Handler<EditEvent> for MyHandler {
    type Result = ();

    fn handle(&mut self, msg: EditEvent, _: &mut Self::Context) -> Self::Result {
        println!("Event: {:?}", msg.0);
    }
}

impl Handler<LookupEvent> for MyHandler {
    type Result = Result<Event, FrontendError>;

    fn handle(&mut self, _: LookupEvent, _: &mut Self::Context) -> Self::Result {
        Err(FrontendErrorKind::Canceled.into())
    }
}

fn main() {
    let sys = System::new("womp");

    event_web::start(MyHandler.start(), "127.0.0.1:8000", None);

    sys.run();
}
