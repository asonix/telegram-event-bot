use std::collections::HashSet;

use actix::ResponseType;
use telebot::objects::Integer;

use error::EventError;
use super::{DeleteState, UserState};

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TouchUser(pub Integer, pub Integer);

impl ResponseType for TouchUser {
    type Item = UserState;
    type Error = EventError;
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct LookupChats(pub Integer);

impl ResponseType for LookupChats {
    type Item = HashSet<Integer>;
    type Error = EventError;
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct LookupChannels(pub Integer);

impl ResponseType for LookupChannels {
    type Item = HashSet<Integer>;
    type Error = EventError;
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct TouchChannel(pub Integer, pub Integer);

impl ResponseType for TouchChannel {
    type Item = ();
    type Error = ();
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct RemoveRelation(pub Integer, pub Integer);

impl ResponseType for RemoveRelation {
    type Item = DeleteState;
    type Error = EventError;
}
