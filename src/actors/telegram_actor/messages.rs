use actix::ResponseType;
use telebot::objects::Update;
use telebot::RcBot;

use models::event::Event;

pub struct TgUpdate {
    pub bot: RcBot,
    pub update: Update,
}

impl ResponseType for TgUpdate {
    type Item = ();
    type Error = ();
}

pub struct StartStreaming;

impl ResponseType for StartStreaming {
    type Item = ();
    type Error = ();
}

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
