use actix::ResponseType;

use models::event::Event;

pub struct NotifyEvent(pub Event);

impl ResponseType for NotifyEvent {
    type Item = ();
    type Error = ();
}
