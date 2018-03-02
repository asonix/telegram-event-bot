use std::collections::HashSet;

use actix::ResponseType;
use telebot::objects::Integer;

use models::event::Event;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct NotifyEvent(pub Event);

impl ResponseType for NotifyEvent {
    type Item = ();
    type Error = ();
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EventOver {
    pub event_id: i32,
    pub system_id: i32,
}

impl ResponseType for EventOver {
    type Item = ();
    type Error = ();
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AskChats(pub HashSet<Integer>, pub Integer);

impl ResponseType for AskChats {
    type Item = ();
    type Error = ();
}
