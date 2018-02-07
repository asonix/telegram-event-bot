#![feature(conservative_impl_trait)]

extern crate actix;
extern crate dotenv;
extern crate event_bot;
extern crate futures;
extern crate telebot;
extern crate tokio_core;

use actix::{Actor, Address, System};
use dotenv::dotenv;
use event_bot::actors::db_broker::DbBroker;
use event_bot::actors::telegram_actor::TelegramActor;
use event_bot::conn::prepare_database_connection;
use event_bot::error::{EventError, EventErrorKind};
use event_bot::telegram::handle_update;
use futures::{Future, Stream};
use telebot::RcBot;
use telebot::objects::Update;

use std::env;

fn bot_token() -> String {
    dotenv().ok();

    env::var("TELEGRAM_BOT_TOKEN").unwrap()
}

fn bot_stream(bot: RcBot) -> impl Stream<Item = (RcBot, Update), Error = EventError> {
    use telebot::functions::*;
    use futures::stream::repeat;
    use futures::stream::iter_ok;

    repeat::<RcBot, EventError>(bot)
        .and_then(move |bot| {
            bot.get_updates()
                .offset(bot.inner.last_id.get())
                .timeout(bot.inner.timeout.get() as i64)
                .send()
                .map_err(|e| e.context(EventErrorKind::Telegram).into())
        })
        .map(|(bot, updates)| iter_ok(updates.0).map(move |update| (bot.clone(), update)))
        .flatten()
        .and_then(move |(bot, update)| {
            if bot.inner.last_id.get() < update.update_id as u32 + 1 {
                bot.inner.last_id.set(update.update_id as u32 + 1);
            }

            Ok((bot, update))
        })
        .filter_map(|(bot, mut update)| {
            let mut forward: Option<String> = None;

            if let Some(ref mut message) = update.message {
                if let Some(text) = message.text.clone() {
                    let mut content = text.split_whitespace();
                    if let Some(cmd) = content.next() {
                        if bot.inner.handlers.borrow_mut().contains_key(cmd) {
                            message.text = Some(content.collect::<Vec<&str>>().join(" "));

                            forward = Some(cmd.into());
                        }
                    }
                }
            }

            if let Some(cmd) = forward {
                if let Some(sender) = bot.inner.handlers.borrow_mut().get_mut(&cmd) {
                    if let Some(msg) = update.message {
                        sender
                            .unbounded_send((bot.clone(), msg))
                            .unwrap_or_else(|e| println!("Error: {}", e));
                    }
                }
                return None;
            } else {
                return Some((bot.clone(), update));
            }
        })
}

fn main() {
    let sys = System::new("tg-event-system");
    let db_url = prepare_database_connection().unwrap();

    let bot_token = bot_token();

    let actor_bot = RcBot::new(sys.handle().clone(), &bot_token);
    let bot = RcBot::new(sys.handle().clone(), &bot_token).timeout(30);

    let bot_stream = bot_stream(bot);

    let db_broker: Address<_> = DbBroker::new(db_url, 10).start();

    let tg: Address<_> = TelegramActor::new(actor_bot, db_broker.clone()).start();

    let fut = bot_stream
        .for_each(move |(bot, update)| {
            handle_update(bot, update, db_broker.clone(), tg.clone());
            Ok(())
        })
        .map_err(|_| ());

    sys.handle().spawn(fut);

    sys.run();
}
