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
extern crate serde;
#[macro_use]
extern crate serde_derive;
extern crate serde_json;
extern crate telebot;
extern crate time;
extern crate tokio_core;
extern crate tokio_postgres;

mod actors;
mod conn;
mod error;
mod models;
mod util;

use actix::{Actor, Addr, Arbiter, Supervisor, Syn, System, Unsync};
use dotenv::dotenv;
use actors::db_broker::DbBroker;
use actors::event_actor::EventActor;
use actors::telegram_actor::TelegramActor;
use actors::telegram_actor::messages::StartStreaming;
use actors::timer::Timer;
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

    let db_broker: Addr<Unsync, _> = DbBroker::new(db_url, 10).start();
    let db_broker2 = db_broker.clone();

    let users_actor = UsersActor::new(db_broker.clone()).start();

    let bot = RcBot::new(Arbiter::handle().clone(), &bot_token()).timeout(30);

    let telegram_actor: Addr<Syn, _> =
        Supervisor::start(move |_| TelegramActor::new(url(), bot, db_broker2, users_actor));

    telegram_actor.do_send(StartStreaming);

    let timer: Addr<Syn, _> = Timer::new(db_broker.clone(), telegram_actor.clone()).start();

    let sync_event_actor: Addr<Syn, _> = EventActor::new(telegram_actor, db_broker, timer).start();
    event_web::start(sync_event_actor, "0.0.0.0:8000", None);

    sys.run();
}
