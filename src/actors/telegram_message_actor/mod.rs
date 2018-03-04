use actix::{Address, Arbiter};
use futures::Future;
use telebot::objects::{CallbackQuery, Integer, Message, Update};
use telebot::RcBot;
use base_x::encode;
use event_web::generate_secret;
use rand::Rng;
use rand::os::OsRng;

use ENCODING_ALPHABET;
use actors::db_actor::messages::StoreEventLink;
use actors::db_actor::messages::{DeleteUserByUserId, LookupSystemByChannel, LookupUser,
                                 NewChannel, NewChat, NewRelation, NewUser, RemoveUserChat};
use actors::db_broker::DbBroker;
use actors::telegram_actor::TelegramActor;
use actors::telegram_actor::messages::{AskChats, CreatedChannel, IsAdmin, Linked, PrintId, SendUrl};
use actors::users_actor::{DeleteState, UserState, UsersActor};
use actors::users_actor::messages::{LookupChannels, RemoveRelation, TouchChannel, TouchUser};
use error::EventErrorKind;
use util::flatten;

mod actor;
mod messages;

pub use self::messages::StartStreaming;

pub struct TelegramMessageActor {
    url: String,
    bot: RcBot,
    db: Address<DbBroker>,
    tg: Address<TelegramActor>,
    users: Address<UsersActor>,
}

impl TelegramMessageActor {
    pub fn new(
        url: String,
        bot: RcBot,
        db: Address<DbBroker>,
        tg: Address<TelegramActor>,
        users: Address<UsersActor>,
    ) -> Self {
        TelegramMessageActor {
            url,
            bot,
            db,
            tg,
            users,
        }
    }

    fn handle_update(&self, update: Update) {
        debug!("handle update");
        if let Some(msg) = update.message {
            self.handle_message(msg);
        } else if let Some(channel_post) = update.channel_post {
            self.handle_channel_post(channel_post);
        } else if let Some(callback_query) = update.callback_query {
            self.handle_callback_query(callback_query);
        } else {
            debug!("Update: {:?}", update);
        }
    }

    fn handle_message(&self, message: Message) {
        debug!("handle message");
        if let Some(user) = message.left_chat_member {
            debug!("left chat member");
            if message.chat.kind == "group" || message.chat.kind == "supergroup" {
                debug!("group | supergroup");
                let chat_id = message.chat.id;
                let user_id = user.id;

                let db = self.db.clone();

                Arbiter::handle().spawn(
                    self.users
                        .call_fut(RemoveRelation(user_id, chat_id))
                        .then(flatten::<RemoveRelation>)
                        .and_then(move |delete_state| {
                            match delete_state {
                                DeleteState::UserEmpty => Arbiter::handle().spawn(
                                    db.call_fut(DeleteUserByUserId(user_id))
                                        .then(flatten::<DeleteUserByUserId>)
                                        .map_err(|e| error!("Error: {:?}", e)),
                                ),
                                _ => (),
                            }

                            db.call_fut(RemoveUserChat(user_id, chat_id))
                                .then(flatten::<RemoveUserChat>)
                        })
                        .map_err(|e| error!("Error: {:?}", e))
                        .map(|_| ()),
                );
            }
        } else if let Some(user) = message.new_chat_member {
            debug!("new chat member");
            if message.chat.kind == "group" || message.chat.kind == "supergroup" {
                debug!("group | supergroup");
                let db = self.db.clone();

                let user_id = user.id;
                let chat_id = message.chat.id;

                Arbiter::handle().spawn(
                    self.users
                        .call_fut(TouchUser(user_id, chat_id))
                        .then(flatten::<TouchUser>)
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
            }
        } else if let Some(user) = message.from {
            debug!("user");
            if let Some(text) = message.text {
                debug!("text");
                if text.starts_with("/new") {
                    debug!("new");
                    if message.chat.kind == "private" {
                        debug!("private");
                        let tg = self.tg.clone();
                        let chat_id = message.chat.id;

                        Arbiter::handle().spawn(
                            self.users
                                .call_fut(LookupChannels(user.id))
                                .then(flatten::<LookupChannels>)
                                .map(move |chats| tg.send(AskChats(chats, chat_id)))
                                .map_err(|e| error!("Error: {:?}", e)),
                        );
                    }
                } else if text.starts_with("/id") {
                    debug!("id");
                    if message.chat.kind == "group" || message.chat.kind == "supergroup" {
                        debug!("group | supergroup");
                        let chat_id = message.chat.id;

                        self.tg.send(PrintId(chat_id));
                    }
                } else {
                    debug!("else");
                    if message.chat.kind == "group" || message.chat.kind == "supergroup" {
                        debug!("group | supergroup");
                        let db = self.db.clone();

                        let user_id = user.id;
                        let chat_id = message.chat.id;

                        Arbiter::handle().spawn(
                            self.users
                                .call_fut(TouchUser(user_id, chat_id))
                                .then(flatten::<TouchUser>)
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
                    }
                }
            }
        }
    }

    fn handle_channel_post(&self, message: Message) {
        debug!("handle channel post");
        if let Some(text) = message.text {
            debug!("text");
            if text.starts_with("/link") {
                debug!("link");
                if message.chat.kind == "channel" {
                    debug!("channel");
                    let db = self.db.clone();
                    let tg = self.tg.clone();
                    let channel_id = message.chat.id;

                    let chat_ids = text.trim_left_matches("/link")
                        .split(' ')
                        .into_iter()
                        .filter_map(|chat_id| chat_id.parse::<Integer>().ok())
                        .map(|chat_id| {
                            self.users.send(TouchChannel(channel_id, chat_id));

                            chat_id
                        })
                        .collect();

                    Arbiter::handle().spawn(
                        self.tg
                            .call_fut(IsAdmin(channel_id, chat_ids))
                            .then(flatten::<IsAdmin>)
                            .and_then(move |chat_ids| {
                                for chat_id in chat_ids.iter() {
                                    db.send(NewChat {
                                        channel_id: channel_id,
                                        chat_id: *chat_id,
                                    });
                                }

                                tg.send(Linked(channel_id, chat_ids));
                                Ok(())
                            })
                            .map_err(|e| error!("Error: {:?}", e)),
                    );
                }
            } else if text.starts_with("/init") {
                debug!("init");
                if message.chat.kind == "channel" {
                    debug!("channel");
                    let channel_id = message.chat.id;
                    let tg = self.tg.clone();

                    Arbiter::handle().spawn(
                        self.db
                            .call_fut(NewChannel { channel_id })
                            .then(flatten::<NewChannel>)
                            .map(move |_chat_system| tg.send(CreatedChannel(channel_id)))
                            .map_err(|e| error!("Error: {:?}", e)),
                    );
                }
            }
        }
    }

    fn handle_callback_query(&self, callback_query: CallbackQuery) {
        debug!("handle callback query");

        let user_id = callback_query.from.id;

        if let Some(msg) = callback_query.message {
            let chat_id = msg.chat.id;

            if let Some(data) = callback_query.data {
                if let Ok(channel_id) = data.parse::<Integer>() {
                    if let Ok(mut rng) = OsRng::new() {
                        let mut bytes = [0; 8];

                        rng.fill_bytes(&mut bytes);
                        let base64d = encode(ENCODING_ALPHABET, &bytes);

                        if let Ok(secret) = generate_secret(&base64d) {
                            let db = self.db.clone();
                            let db2 = self.db.clone();
                            let tg = self.tg.clone();
                            let users = self.users.clone();

                            let url = self.url.clone();

                            Arbiter::handle().spawn(
                                self.db
                                    .call_fut(LookupUser(user_id))
                                    .then(flatten::<LookupUser>)
                                    .and_then(move |user| {
                                        db.call_fut(LookupSystemByChannel(channel_id))
                                            .then(flatten::<LookupSystemByChannel>)
                                            .map(|chat_system| (chat_system, user))
                                    })
                                    .and_then(move |(chat_system, user)| {
                                        let events_channel = chat_system.events_channel();
                                        users
                                            .call_fut(LookupChannels(user.user_id()))
                                            .then(flatten::<LookupChannels>)
                                            .and_then(move |channel_ids| {
                                                if channel_ids.contains(&events_channel) {
                                                    Ok(())
                                                } else {
                                                    Err(EventErrorKind::Permissions.into())
                                                }
                                            })
                                            .and_then(move |_| {
                                                db2.call_fut(StoreEventLink {
                                                    user_id: user.id(),
                                                    system_id: chat_system.id(),
                                                    secret,
                                                }).then(flatten::<StoreEventLink>)
                                                    .map(move |_| user)
                                            })
                                    })
                                    .map(move |user| {
                                        tg.send(SendUrl(
                                            chat_id,
                                            format!("{}/events/new/{}={}", url, base64d, user.id()),
                                        ))
                                    })
                                    .map_err(|e| error!("Error: {:?}", e)),
                            );
                        }
                    }
                }
            }
        }
    }
}
