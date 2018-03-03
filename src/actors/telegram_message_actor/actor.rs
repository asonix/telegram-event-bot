use actix::{Actor, ActorContext, Address, Arbiter, AsyncContext, Context, Handler, Supervised};
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

    fn started(&mut self, _: &mut Self::Context) {
        debug!("Started telegram message actor");
    }
}

impl Supervised for TelegramMessageActor {
    fn restarting(&mut self, _: &mut <Self as Actor>::Context) {
        debug!("Restarting telegram message actor!");
        self.bot = RcBot::new(Arbiter::handle().clone(), &self.bot.inner.key);
    }
}

impl Handler<Result<TgUpdate, EventError>> for TelegramMessageActor {
    type Result = ();

    fn handle(
        &mut self,
        msg: Result<TgUpdate, EventError>,
        ctx: &mut Self::Context,
    ) -> Self::Result {
        debug!("Handling update");
        match msg {
            Ok(tg_update) => self.handle_update(tg_update.update),
            Err(err) => {
                error!("Error {:?}", err);
                ctx.stop();
            }
        }
    }
}

impl Handler<StartStreaming> for TelegramMessageActor {
    type Result = ();

    fn handle(&mut self, _: StartStreaming, ctx: &mut Self::Context) -> Self::Result {
        let addr: Address<_> = ctx.address();

        Arbiter::handle().spawn(
            bot_stream(self.bot.clone())
                .then(move |res| match res {
                    Ok((bot, update)) => addr.call_fut(Ok(TgUpdate { bot, update })),
                    Err(e) => addr.call_fut(Err(e)),
                })
                .map_err(|e| error!("Error: {:?}", e))
                .for_each(|_| Ok(())),
        )
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
                            .unwrap_or_else(|e| error!("Error: {}", e));
                    }
                }
                return None;
            } else {
                return Some((bot.clone(), update));
            }
        })
}
