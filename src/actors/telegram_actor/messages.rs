use actix::ResponseType;
use telebot::objects::Integer;

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

pub struct AnswerTitle {
    pub chat_id: Integer,
}

impl ResponseType for AnswerTitle {
    type Item = ();
    type Error = ();
}

pub struct FailedAnswerDescription {
    pub chat_id: Integer,
}

impl ResponseType for FailedAnswerDescription {
    type Item = ();
    type Error = ();
}

pub struct AnswerDescription {
    pub chat_id: Integer,
}

impl ResponseType for AnswerDescription {
    type Item = ();
    type Error = ();
}

pub struct FailedAnswerDate {
    pub chat_id: Integer,
}

impl ResponseType for FailedAnswerDate {
    type Item = ();
    type Error = ();
}

pub struct AnswerDate {
    pub chat_id: Integer,
}

impl ResponseType for AnswerDate {
    type Item = ();
    type Error = ();
}

pub struct FailedAnswerEnd {
    pub chat_id: Integer,
}

impl ResponseType for FailedAnswerEnd {
    type Item = ();
    type Error = ();
}

pub struct AnswerEnd {
    pub chat_id: Integer,
}

impl ResponseType for AnswerEnd {
    type Item = ();
    type Error = ();
}

pub struct FailedAnswerHost {
    pub chat_id: Integer,
}

impl ResponseType for FailedAnswerHost {
    type Item = ();
    type Error = ();
}

pub struct AnswerHost {
    pub chat_id: Integer,
}

impl ResponseType for AnswerHost {
    type Item = ();
    type Error = ();
}

pub struct FailedAnswerFinalize {
    pub chat_id: Integer,
}

impl ResponseType for FailedAnswerFinalize {
    type Item = ();
    type Error = ();
}

pub struct AnswerFinalize {
    pub chat_id: Integer,
}

impl ResponseType for AnswerFinalize {
    type Item = ();
    type Error = ();
}
