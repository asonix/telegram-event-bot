// use futures::{Future, IntoFuture};
use actix::Address;
use telebot::RcBot;
use telebot::objects::{Message, Update};

use actors::db_broker::DbBroker;
use actors::db_actor::messages::NewUser;
use actors::telegram_actor::TelegramActor;

pub fn handle_update(
    bot: RcBot,
    update: Update,
    db: Address<DbBroker>,
    tg: Address<TelegramActor>,
) {
    if let Some(msg) = update.message {
        handle_message(bot, msg, db, tg);
    }
}

fn handle_message(bot: RcBot, message: Message, db: Address<DbBroker>, tg: Address<TelegramActor>) {
    if let Some(user) = message.from {
        if message.chat.kind == "group" || message.chat.kind == "supergroup" {
            db.send(NewUser {
                chat_id: message.chat.id,
                user_id: user.id,
            })
        }
    }
}
