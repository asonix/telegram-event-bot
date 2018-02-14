#![feature(conservative_impl_trait)]

extern crate actix;
extern crate dotenv;
extern crate event_bot;
extern crate futures;
extern crate telebot;
extern crate tokio_core;

use actix::{Actor, Address, Arbiter, System};
use dotenv::dotenv;
use event_bot::actors::db_broker::DbBroker;
use event_bot::actors::telegram_actor::TelegramActor;
use event_bot::actors::telegram_message_actor::TelegramMessageActor;
use event_bot::conn::prepare_database_connection;
use telebot::RcBot;

use std::env;

fn bot_token() -> String {
    dotenv().ok();

    env::var("TELEGRAM_BOT_TOKEN").unwrap()
}

fn main() {
    let sys = System::new("tg-event-system");
    let _ = Arbiter::new("one");

    let db_url = prepare_database_connection().unwrap();

    let bot_token = bot_token();

    let actor_bot = RcBot::new(Arbiter::handle().clone(), &bot_token);

    let db_broker: Address<_> = DbBroker::new(db_url, 10).start();

    let tg: Address<_> = TelegramActor::new(actor_bot, db_broker.clone()).start();

    let msg_actor_bot = RcBot::new(Arbiter::handle().clone(), &bot_token);
    let _: Address<_> = TelegramMessageActor::new(msg_actor_bot, db_broker, tg).start();

    sys.run();
}
