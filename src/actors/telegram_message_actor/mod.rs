use actix::{Address, Arbiter};
use failure::Fail;
use futures::Future;
use telebot::objects::{Message, Update};
use telebot::RcBot;

use actors::db_actor::messages::{NewRelation, NewUser};
use actors::db_broker::DbBroker;
use actors::telegram_actor::TelegramActor;
use actors::users_actor::{UserState, UsersActor};
use actors::users_actor::messages::TouchUser;
use error::EventErrorKind;

mod actor;
mod messages;

pub struct TelegramMessageActor {
    bot: RcBot,
    db: Address<DbBroker>,
    tg: Address<TelegramActor>,
    users: Address<UsersActor>,
}

impl TelegramMessageActor {
    pub fn new(
        bot: RcBot,
        db: Address<DbBroker>,
        tg: Address<TelegramActor>,
        users: Address<UsersActor>,
    ) -> Self {
        TelegramMessageActor { bot, db, tg, users }
    }

    fn handle_update(&self, update: Update) {
        if let Some(msg) = update.message {
            self.handle_message(msg);
        }
    }

    fn handle_message(&self, message: Message) {
        if let Some(user) = message.from {
            if message.chat.kind == "group" || message.chat.kind == "supergroup" {
                let db = self.db.clone();

                let user_id = user.id;
                let chat_id = message.chat.id;

                Arbiter::handle().spawn(
                    self.users
                        .call_fut(TouchUser(user_id, chat_id))
                        .then(|msg_res| match msg_res {
                            Ok(res) => res,
                            Err(e) => Err(e.context(EventErrorKind::Canceled).into()),
                        })
                        .and_then(move |user_state| {
                            Ok(match user_state {
                                UserState::NewRelation => {
                                    db.send(NewRelation { chat_id, user_id });
                                }
                                UserState::NewUser => {
                                    db.send(NewUser { chat_id, user_id });
                                }
                                _ => (),
                            })
                        })
                        .map_err(|_| ()),
                );
            }
        }
    }
}
