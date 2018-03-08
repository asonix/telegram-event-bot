use std::collections::HashSet;

use actix::ResponseType;
use telebot::objects::Integer;

use error::EventError;
use models::event::Event;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EventSoon(pub Event);

impl ResponseType for EventSoon {
    type Item = ();
    type Error = ();
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EventStarted(pub Event);

impl ResponseType for EventStarted {
    type Item = ();
    type Error = ();
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EventOver(pub Event);

impl ResponseType for EventOver {
    type Item = ();
    type Error = ();
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct NewEvent(pub Event);

impl ResponseType for NewEvent {
    type Item = ();
    type Error = ();
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct UpdateEvent(pub Event);

impl ResponseType for UpdateEvent {
    type Item = ();
    type Error = ();
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AskChats(pub HashSet<Integer>, pub Integer);

impl ResponseType for AskChats {
    type Item = ();
    type Error = ();
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AskEvents(pub Vec<Event>, pub Integer);

impl ResponseType for AskEvents {
    type Item = ();
    type Error = ();
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct IsAdmin(pub Integer, pub Vec<Integer>);

impl ResponseType for IsAdmin {
    type Item = Vec<Integer>;
    type Error = EventError;
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Linked(pub Integer, pub Vec<Integer>);

impl ResponseType for Linked {
    type Item = ();
    type Error = ();
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PrintId(pub Integer);

impl ResponseType for PrintId {
    type Item = ();
    type Error = ();
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct CreatedChannel(pub Integer);

impl ResponseType for CreatedChannel {
    type Item = ();
    type Error = ();
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SendUrl(pub Integer, pub String, pub String);

impl ResponseType for SendUrl {
    type Item = ();
    type Error = ();
}
