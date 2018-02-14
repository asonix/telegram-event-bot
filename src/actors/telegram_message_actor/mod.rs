use actix::Address;
use telebot::objects::{Message, Update};
use telebot::RcBot;

use actors::db_actor::messages::NewUser;
use actors::db_broker::DbBroker;
use actors::telegram_actor::TelegramActor;

mod actor;
mod messages;

pub struct TelegramMessageActor {
    bot: RcBot,
    db: Address<DbBroker>,
    tg: Address<TelegramActor>,
}

impl TelegramMessageActor {
    pub fn new(bot: RcBot, db: Address<DbBroker>, tg: Address<TelegramActor>) -> Self {
        TelegramMessageActor { bot, db, tg }
    }

    fn handle_update(&self, update: Update) {
        if let Some(msg) = update.message {
            self.handle_message(msg);
        }
    }

    fn handle_message(&self, message: Message) {
        if let Some(user) = message.from {
            if message.chat.kind == "group" || message.chat.kind == "supergroup" {
                self.db.send(NewUser {
                    chat_id: message.chat.id,
                    user_id: user.id,
                })
            }
        }
    }
}
