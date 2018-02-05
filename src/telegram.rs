// use futures::{Future, IntoFuture};
use telebot::RcBot;
use telebot::objects::{Message, Update};

pub fn handle_update(bot: RcBot, update: Update) {
    if let Some(msg) = update.message {
        handle_message(bot, msg);
    }
}

fn handle_message(bot: RcBot, message: Message) {}
