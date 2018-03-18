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

//! This module defines the functionality for the UsersActor

use std::collections::{HashMap, HashSet};

use actix::Address;
use telebot::objects::Integer;

use actors::db_broker::DbBroker;

mod actor;
pub mod messages;

/// `UserState` is used to track whether a relation between a user and a chat is new, or known, or
/// whether a user is new entirely.
pub enum UserState {
    NewUser,
    NewRelation,
    KnownRelation,
    InvalidQuery,
}

/// `DeleteState` is used to track whether a releation deletion request has completely emptied a
/// user's chats, or whether the user is still valid.
pub enum DeleteState {
    UserValid,
    UserEmpty,
}

/// The UsersActor handles keeping information on user/chat and chat/channel relations in-memory
/// for faster lookups
pub struct UsersActor {
    // maps user_id to HashSet<ChatId>
    users: HashMap<Integer, HashSet<Integer>>,
    // maps channel_id to HashSet<ChatId>
    channels: HashMap<Integer, HashSet<Integer>>,
    chats: HashSet<Integer>,
    db: Address<DbBroker>,
}

impl UsersActor {
    pub fn new(db: Address<DbBroker>) -> Self {
        UsersActor {
            users: HashMap::new(),
            channels: HashMap::new(),
            chats: HashSet::new(),
            db: db,
        }
    }

    fn touch_user(&mut self, user_id: Integer, chat_id: Integer) -> UserState {
        if !self.chats.contains(&chat_id) {
            return UserState::InvalidQuery;
        }

        let exists = self.users.contains_key(&user_id);

        if exists {
            if self.users
                .entry(user_id)
                .or_insert(HashSet::new())
                .insert(chat_id)
            {
                UserState::NewRelation
            } else {
                UserState::KnownRelation
            }
        } else {
            self.users
                .entry(user_id)
                .or_insert(HashSet::new())
                .insert(chat_id);
            UserState::NewUser
        }
    }

    fn touch_channel(&mut self, channel_id: Integer, chat_id: Integer) {
        self.chats.insert(chat_id);

        self.channels
            .entry(channel_id)
            .or_insert(HashSet::new())
            .insert(chat_id);
    }

    fn lookup_chats(&mut self, user_id: Integer) -> HashSet<Integer> {
        self.users
            .get(&user_id)
            .map(|chats| chats.clone())
            .unwrap_or(HashSet::new())
    }

    fn lookup_channels(&mut self, user_id: Integer) -> HashSet<Integer> {
        self.lookup_chats(user_id)
            .into_iter()
            .filter_map(|chat_id| {
                self.channels
                    .iter()
                    .find(|&(_, ref chat_hash_set)| chat_hash_set.contains(&chat_id))
                    .map(|(k, _)| *k)
            })
            .collect()
    }

    fn remove_relation(&mut self, user_id: Integer, chat_id: Integer) -> DeleteState {
        debug!("Removing chat {} from user {}", chat_id, user_id);
        let mut hs = match self.users.remove(&user_id) {
            Some(hs) => hs,
            None => return DeleteState::UserEmpty,
        };

        hs.remove(&chat_id);

        if !hs.is_empty() {
            self.users.insert(user_id, hs);
            DeleteState::UserValid
        } else {
            DeleteState::UserEmpty
        }
    }
}
