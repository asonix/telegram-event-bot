use actix::{Actor, ActorContext, AsyncContext, Context, Handler};
use futures::{Future, Stream};
use futures::stream::{iter_ok, repeat};
use telebot::functions::*;
use telebot::objects::Update;
use telebot::RcBot;

use error::{EventError, EventErrorKind};
use super::messages::*;
use super::TelegramMessageActor;

impl Actor for TelegramMessageActor {
    type Context = Context<Self>;

    fn started(&mut self, ctx: &mut Self::Context) {
        ctx.add_stream(bot_stream(self.bot.clone()).map(|(bot, update)| TgUpdate { bot, update }));
    }
}

impl Handler<Result<TgUpdate, EventError>> for TelegramMessageActor {
    type Result = ();

    fn handle(
        &mut self,
        msg: Result<TgUpdate, EventError>,
        ctx: &mut Self::Context,
    ) -> Self::Result {
        match msg {
            Ok(tg_update) => self.handle_update(tg_update.update),
            Err(err) => {
                println!("Error {:?}", err);
                ctx.stop();
            }
        }
    }
}

fn bot_stream(bot: RcBot) -> impl Stream<Item = (RcBot, Update), Error = EventError> {
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
