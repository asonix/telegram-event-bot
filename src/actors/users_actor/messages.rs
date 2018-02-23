use actix::ResponseType;
use telebot::objects::Integer;

use error::EventError;
use super::UserState;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TouchUser(pub Integer, pub Integer);

impl ResponseType for TouchUser {
    type Item = UserState;
    type Error = EventError;
}
