/*
 * This file is part of Telegram Event Bot.
 *
 * Copyright © 2018 Riley Trautman
 *
 * Telegram Event Bot is free software: you can redistribute it and/or modify
 * it under the terms of the GNU General Public License as published by
 * the Free Software Foundation, either version 3 of the License, or
 * (at your option) any later version.
 *
 * Telegram Event Bot is distributed in the hope that it will be useful,
 * but WITHOUT ANY WARRANTY; without even the implied warranty of
 * MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
 * GNU General Public License for more details.
 *
 * You should have received a copy of the GNU General Public License
 * along with Telegram Event Bot.  If not, see <http://www.gnu.org/licenses/>.
 */

//! This module defines all messages that the UsersActor can receive

use std::collections::HashSet;

use actix::ResponseType;
use telebot::objects::Integer;

use error::EventError;
use super::{DeleteState, UserState};

/// This type is for ensuring a releationship between a user and a chat
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TouchUser(pub Integer, pub Integer);

impl ResponseType for TouchUser {
    type Item = UserState;
    type Error = EventError;
}

/// This type is for looking up chats for a given user
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct LookupChats(pub Integer);

impl ResponseType for LookupChats {
    type Item = HashSet<Integer>;
    type Error = EventError;
}

/// This type is for looking up channels for a given user
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct LookupChannels(pub Integer);

impl ResponseType for LookupChannels {
    type Item = HashSet<Integer>;
    type Error = EventError;
}

/// This type is for ensuring a relationship between a channel and a chat
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct TouchChannel(pub Integer, pub Integer);

impl ResponseType for TouchChannel {
    type Item = ();
    type Error = ();
}

/// This type is for removing a user from a chat
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct RemoveRelation(pub Integer, pub Integer);

impl ResponseType for RemoveRelation {
    type Item = DeleteState;
    type Error = EventError;
}
