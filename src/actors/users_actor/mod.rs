use std::collections::{HashMap, HashSet};

use actix::Address;
use telebot::objects::Integer;

use actors::db_broker::DbBroker;

mod actor;
pub mod messages;

pub enum UserState {
    NewUser,
    NewRelation,
    KnownRelation,
}

pub enum DeleteState {
    UserValid,
    UserEmpty,
}

pub struct UsersActor {
    // maps user_id to HashSet<ChatId>
    users: HashMap<Integer, HashSet<Integer>>,
    // maps channel_id to HashSet<ChatId>
    channels: HashMap<Integer, HashSet<Integer>>,
    db: Address<DbBroker>,
}

impl UsersActor {
    pub fn new(db: Address<DbBroker>) -> Self {
        UsersActor {
            users: HashMap::new(),
            channels: HashMap::new(),
            db: db,
        }
    }

    fn touch_user(&mut self, user_id: Integer, chat_id: Integer) -> UserState {
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
