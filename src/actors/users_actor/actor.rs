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

//! This module defines the actor-related behaviours for the UsersActor

use std::collections::HashSet;

use actix::{Actor, AsyncContext, Context, Handler, Message, Running, StreamHandler};
use futures::stream::iter_ok;
use futures::{Future, Stream};
use telebot::objects::Integer;

use super::messages::*;
use super::{DeleteState, UsersActor};
use actors::db_broker::messages::{GetSystemsWithChats, GetUsersWithChats};
use error::EventError;
use models::chat::Chat;
use models::chat_system::ChatSystem;
use models::user::User;
use util::flatten;

impl Actor for UsersActor {
    type Context = Context<Self>;

    fn started(&mut self, ctx: &mut Self::Context) {
        let db = self.db.clone();

        // add a stream that adds channels from the database to the UsersActor's store
        ctx.add_stream(
            db.send(GetSystemsWithChats)
                .then(flatten)
                .into_stream()
                .and_then(|systems_with_chats: Vec<(ChatSystem, Chat)>| {
                    Ok(iter_ok(systems_with_chats.into_iter().map(|(s, c)| {
                        TouchChannel(s.events_channel(), c.chat_id())
                    })))
                })
                .flatten(),
        );
    }
}

impl StreamHandler<TouchUser, EventError> for UsersActor {
    fn handle(&mut self, msg: TouchUser, _: &mut Self::Context) {
        self.touch_user(msg.0, msg.1);
    }

    fn error(&mut self, err: EventError, _: &mut Self::Context) -> Running {
        error!("Error in TouchUser: {:?}", err);
        Running::Continue
    }

    fn finished(&mut self, _: &mut Self::Context) {
        debug!("Done importing Users");
    }
}

impl StreamHandler<TouchChannel, EventError> for UsersActor {
    fn handle(&mut self, msg: TouchChannel, _: &mut Self::Context) {
        self.touch_channel(msg.0, msg.1);
    }

    fn error(&mut self, err: EventError, _: &mut Self::Context) -> Running {
        error!("Error in TouchChannel: {:?}", err);
        Running::Continue
    }

    fn finished(&mut self, ctx: &mut Self::Context) {
        debug!("Done importing Channels");
        let db = self.db.clone();

        // add a stream that adds users from the database to the UsersActor's store
        ctx.add_stream(
            db.send(GetUsersWithChats)
                .then(flatten)
                .into_stream()
                .and_then(|users_with_chats: Vec<(User, Chat)>| {
                    Ok(iter_ok(
                        users_with_chats
                            .into_iter()
                            .map(|(u, c)| TouchUser(u.user_id(), c.chat_id())),
                    ))
                })
                .flatten(),
        );
    }
}

impl Handler<TouchUser> for UsersActor {
    type Result = <TouchUser as Message>::Result;

    fn handle(&mut self, msg: TouchUser, _: &mut Self::Context) -> Self::Result {
        Ok(self.touch_user(msg.0, msg.1))
    }
}

impl Handler<TouchChannel> for UsersActor {
    type Result = <TouchChannel as Message>::Result;

    fn handle(&mut self, msg: TouchChannel, _: &mut Self::Context) -> Self::Result {
        self.touch_channel(msg.0, msg.1)
    }
}

impl Handler<LookupChats> for UsersActor {
    type Result = Result<HashSet<Integer>, EventError>;

    fn handle(&mut self, msg: LookupChats, _: &mut Self::Context) -> Self::Result {
        Ok(self.lookup_chats(msg.0))
    }
}

impl Handler<LookupChannels> for UsersActor {
    type Result = Result<HashSet<Integer>, EventError>;

    fn handle(&mut self, msg: LookupChannels, _: &mut Self::Context) -> Self::Result {
        Ok(self.lookup_channels(msg.0))
    }
}

impl Handler<RemoveRelation> for UsersActor {
    type Result = Result<DeleteState, EventError>;

    fn handle(&mut self, msg: RemoveRelation, _: &mut Self::Context) -> Self::Result {
        Ok(self.remove_relation(msg.0, msg.1))
    }
}
