#![feature(conservative_impl_trait)]

extern crate actix;
extern crate dotenv;
extern crate env_logger;
extern crate event_bot;
extern crate event_web;
extern crate futures;
extern crate telebot;
extern crate tokio_core;

use actix::{Actor, Address, Arbiter, System};
use dotenv::dotenv;
use event_bot::actors::db_broker::DbBroker;
use event_bot::actors::event_actor::EventActor;
use event_bot::actors::telegram_actor::TelegramActor;
use event_bot::actors::telegram_message_actor::TelegramMessageActor;
use event_bot::actors::users_actor::UsersActor;
use event_bot::conn::prepare_database_connection;
use telebot::RcBot;

use std::env;

fn bot_token() -> String {
    dotenv().ok();

    env::var("TELEGRAM_BOT_TOKEN").unwrap()
}

fn main() {
    env_logger::init();

    let sys = System::new("tg-event-system");
    let _ = Arbiter::new("one");

    let db_url = prepare_database_connection().unwrap();

    let bot_token = bot_token();

    let actor_bot = RcBot::new(Arbiter::handle().clone(), &bot_token);

    let db_broker: Address<_> = DbBroker::new(db_url, 10).start();

    let users_actor = UsersActor::new(db_broker.clone()).start();

    let tg: Address<_> = TelegramActor::new(actor_bot, db_broker.clone()).start();

    let event_actor: Address<_> = EventActor::new(tg.clone(), db_broker.clone()).start();

    let msg_actor_bot = RcBot::new(Arbiter::handle().clone(), &bot_token);
    let _: Address<_> =
        TelegramMessageActor::new(msg_actor_bot, db_broker, tg, users_actor, event_actor).start();

    sys.run();
}
