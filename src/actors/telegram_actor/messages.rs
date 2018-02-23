use actix::ResponseType;

use models::event::Event;

pub struct NotifyEvent(pub Event);

impl ResponseType for NotifyEvent {
    type Item = ();
    type Error = ();
}

pub struct EventOver {
    pub event_id: i32,
    pub system_id: i32,
}

impl ResponseType for EventOver {
    type Item = ();
    type Error = ();
}
