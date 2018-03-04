#![feature(conservative_impl_trait)]

extern crate actix;
extern crate base_x;
extern crate chrono;
extern crate chrono_tz;
extern crate dotenv;
extern crate env_logger;
extern crate event_web;
extern crate failure;
#[macro_use]
extern crate failure_derive;
extern crate futures;
extern crate futures_state_stream;
#[macro_use]
extern crate log;
extern crate rand;
extern crate telebot;
extern crate time;
extern crate tokio_core;
extern crate tokio_postgres;
extern crate tokio_timer;

mod actors;
mod conn;
mod error;
mod models;
mod util;

use actix::{Actor, Address, Arbiter, Supervisor, SyncAddress, System};
use dotenv::dotenv;
use actors::db_broker::DbBroker;
use actors::event_actor::EventActor;
use actors::telegram_actor::TelegramActor;
use actors::telegram_message_actor::{StartStreaming, TelegramMessageActor};
use actors::users_actor::UsersActor;
use conn::prepare_database_connection;
use telebot::RcBot;

use std::env;

const ENCODING_ALPHABET: &str = "abcdefghizklmnopqrstuvwxyz1234567890";

fn bot_token() -> String {
    dotenv().ok();

    env::var("TELEGRAM_BOT_TOKEN").unwrap()
}

fn url() -> String {
    dotenv().ok();

    env::var("EVENT_URL").unwrap()
}

fn main() {
    env_logger::init();

    debug!("Running!");

    let sys = System::new("tg-event-system");
    let _ = Arbiter::new("one");

    let db_url = prepare_database_connection().unwrap();

    let bot_token = bot_token();

    let actor_bot = RcBot::new(Arbiter::handle().clone(), &bot_token);

    let db_broker: Address<_> = DbBroker::new(db_url, 10).start();

    let users_actor = UsersActor::new(db_broker.clone()).start();

    let tg: Address<_> = TelegramActor::new(actor_bot, db_broker.clone()).start();

    let sync_event_actor: SyncAddress<_> = EventActor::new(tg.clone(), db_broker.clone()).start();

    let msg_actor_bot = RcBot::new(Arbiter::handle().clone(), &bot_token);

    event_web::start(sync_event_actor, "127.0.0.1:8000", None);

    let tma: Address<_> = Supervisor::start(|_| {
        TelegramMessageActor::new(url(), msg_actor_bot, db_broker, tg, users_actor)
    });

    tma.send(StartStreaming);

    sys.run();
}
