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

//! This module defines the types that the `TelegramActor` accepts as messages. They come in two
//! classes: Those that the `TelegramActor` sends itself, and those that other actors send.

use actix::ResponseType;
use telebot::objects::Update;
use telebot::RcBot;

use models::event::Event;

/// This message comes when the bot receives an Update or a series of Updates from telegram
///
/// The `TelegramActor` itself manages the stream that produces these.
pub struct TgUpdate {
    pub bot: RcBot,
    pub update: Update,
}

impl ResponseType for TgUpdate {
    type Item = ();
    type Error = ();
}

/// This message instructs the actor to start the Telegram Update stream. It is sent when the actor
/// crashes and restarts, or when the stream errors and needs to restart.
pub struct StartStreaming;

impl ResponseType for StartStreaming {
    type Item = ();
    type Error = ();
}

/// This message is to alert the required channel that an event is starting soon. The Timer actor
/// produces this message
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EventSoon(pub Event);

impl ResponseType for EventSoon {
    type Item = ();
    type Error = ();
}

/// This message is to alert the required channel that an event has started. The Timer actor
/// produces this message
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EventStarted(pub Event);

impl ResponseType for EventStarted {
    type Item = ();
    type Error = ();
}

/// This message is to alert the required channel that an event is over. The Timer actor produces
/// this message
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EventOver(pub Event);

impl ResponseType for EventOver {
    type Item = ();
    type Error = ();
}

/// This message is to alert the require channel that an event has been created.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct NewEvent(pub Event);

impl ResponseType for NewEvent {
    type Item = ();
    type Error = ();
}

/// This message is to alert the required channel that an event has been updated.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct UpdateEvent(pub Event);

impl ResponseType for UpdateEvent {
    type Item = ();
    type Error = ();
}
