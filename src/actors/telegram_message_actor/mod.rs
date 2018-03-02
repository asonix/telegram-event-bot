use actix::{Address, Arbiter};
use failure::Fail;
use futures::Future;
use telebot::objects::{CallbackQuery, Integer, Message, Update};
use telebot::RcBot;

use actors::db_actor::messages::{LookupUser, NewRelation, NewUser};
use actors::db_broker::DbBroker;
use actors::event_actor::EventActor;
use actors::telegram_actor::TelegramActor;
use actors::telegram_actor::messages::AskChats;
use actors::users_actor::{UserState, UsersActor};
use actors::users_actor::messages::{LookupChats, TouchUser};
use error::EventErrorKind;

mod actor;
mod messages;

pub struct TelegramMessageActor {
    bot: RcBot,
    db: Address<DbBroker>,
    tg: Address<TelegramActor>,
    users: Address<UsersActor>,
    event: Address<EventActor>,
}

impl TelegramMessageActor {
    pub fn new(
        bot: RcBot,
        db: Address<DbBroker>,
        tg: Address<TelegramActor>,
        users: Address<UsersActor>,
        event: Address<EventActor>,
    ) -> Self {
        TelegramMessageActor {
            bot,
            db,
            tg,
            users,
            event,
        }
    }

    fn handle_update(&self, update: Update) {
        println!("Update: {:?}", update);

        if let Some(msg) = update.message {
            self.handle_message(msg);
        } else if let Some(callback_query) = update.callback_query {
            self.handle_callback_query(callback_query);
        }
    }

    fn handle_message(&self, message: Message) {
        if let Some(user) = message.from {
            if message.chat.kind == "group" || message.chat.kind == "supergroup" {
                let db = self.db.clone();

                let user_id = user.id;
                let chat_id = message.chat.id;

                Arbiter::handle().spawn(
                    self.users
                        .call_fut(TouchUser(user_id, chat_id))
                        .then(|msg_res| match msg_res {
                            Ok(res) => res,
                            Err(e) => Err(e.context(EventErrorKind::Canceled).into()),
                        })
                        .and_then(move |user_state| {
                            Ok(match user_state {
                                UserState::NewRelation => {
                                    db.send(NewRelation { chat_id, user_id });
                                }
                                UserState::NewUser => {
                                    db.send(NewUser { chat_id, user_id });
                                }
                                _ => (),
                            })
                        })
                        .map_err(|e| error!("Error: {:?}", e)),
                );
            } else {
                if let Some(text) = message.text {
                    if text.starts_with("/new") {
                        let tg = self.tg.clone();
                        let chat_id = message.chat.id;

                        Arbiter::handle().spawn(
                            self.users
                                .call_fut(LookupChats(user.id))
                                .then(|msg_res| match msg_res {
                                    Ok(res) => res,
                                    Err(e) => Err(e.context(EventErrorKind::Canceled).into()),
                                })
                                .map(move |chats| tg.send(AskChats(chats, chat_id)))
                                .map_err(|e| error!("Error: {:?}", e)),
                        );
                    }
                }
            }
        }
    }

    fn handle_callback_query(&self, callback_query: CallbackQuery) {
        use actors::db_actor::messages::StoreEventLink;
        use base64::encode;
        use event_web::generate_secret;
        use rand::Rng;
        use rand::os::OsRng;

        let user_id = callback_query.from.id;

        if let Some(data) = callback_query.data {
            if let Ok(chat_id) = data.parse::<Integer>() {
                if let Ok(mut rng) = OsRng::new() {
                    let mut bytes = [0; 8];

                    rng.fill_bytes(&mut bytes);
                    let base64d = encode(&bytes);

                    if let Ok(secret) = generate_secret(&base64d) {
                        let db = self.db.clone();

                        let fut = self.db
                            .call_fut(LookupUser(user_id))
                            .then(|msg_res| match msg_res {
                                Ok(res) => res,
                                Err(e) => Err(e.context(EventErrorKind::Canceled).into()),
                            })
                            .and_then(move |user| {
                                db.call_fut(StoreEventLink {
                                    user_id: user.id(),
                                    secret,
                                }).then(|msg_res| match msg_res {
                                    Ok(res) => res,
                                    Err(e) => Err(e.context(EventErrorKind::Canceled).into()),
                                })
                            });

                        Arbiter::handle()
                            .spawn(fut.map(|_| ()).map_err(|e| error!("Error: {:?}", e)));
                    }
                }
            }
        }
    }
}
