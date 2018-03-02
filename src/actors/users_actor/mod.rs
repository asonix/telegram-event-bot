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

pub struct UsersActor {
    users: HashMap<Integer, HashSet<Integer>>,
    db: Address<DbBroker>,
}

impl UsersActor {
    pub fn new(db: Address<DbBroker>) -> Self {
        UsersActor {
            users: HashMap::new(),
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

    fn lookup_chats(&mut self, user_id: Integer) -> HashSet<Integer> {
        self.users
            .get(&user_id)
            .map(|chats| chats.clone())
            .unwrap_or(HashSet::new())
    }
}
