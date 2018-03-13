use actix::{Address, Arbiter};
use futures::Future;
use telebot::objects::{CallbackQuery, Integer, Message, Update};
use telebot::RcBot;
use base_x::encode;
use event_web::generate_secret;
use rand::Rng;
use rand::os::OsRng;
use serde_json;

use ENCODING_ALPHABET;
use actors::db_broker::messages::{DeleteEvent, DeleteUserByUserId, LookupEvent,
                                  LookupEventsByChatId, LookupEventsByUserId, LookupSystem,
                                  LookupSystemByChannel, LookupUser, NewChannel, NewChat,
                                  NewRelation, NewUser, RemoveUserChat, StoreEditEventLink,
                                  StoreEventLink};
use actors::db_broker::DbBroker;
use actors::telegram_actor::{CallbackQueryMessage, TelegramActor};
use actors::telegram_actor::messages::{AskChats, AskDeleteEvents, AskEvents, CreatedChannel,
                                       EventDeleted, IsAdmin, Linked, PrintId, SendError,
                                       SendEvents, SendHelp, SendUrl};
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
        debug!("handle update: {}", update.update_id);
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
                let username = user.username.unwrap_or(user.first_name);
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
                                    db.send(NewUser {
                                        chat_id,
                                        user_id,
                                        username,
                                    });
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
                                .then(move |chats| match chats {
                                    Ok(chats) => Ok(tg.send(AskChats(chats, chat_id))),
                                    Err(e) => {
                                        tg.send(SendError(
                                            chat_id,
                                            "Failed to get event channnels for user",
                                        ));
                                        Err(e)
                                    }
                                })
                                .map_err(|e| error!("Error: {:?}", e)),
                        );
                    }
                } else if text.starts_with("/edit") {
                    debug!("edit");
                    if message.chat.kind == "private" {
                        debug!("private");
                        let tg = self.tg.clone();
                        let chat_id = message.chat.id;

                        Arbiter::handle().spawn(
                            self.db
                                .call_fut(LookupEventsByUserId { user_id: user.id })
                                .then(flatten::<LookupEventsByUserId>)
                                .then(move |events| match events {
                                    Ok(events) => Ok(tg.send(AskEvents(events, chat_id))),
                                    Err(e) => {
                                        tg.send(SendError(
                                            chat_id,
                                            "Failed to get events for user",
                                        ));
                                        Err(e)
                                    }
                                })
                                .map_err(|e| error!("Error: {:?}", e)),
                        );
                    }
                } else if text.starts_with("/delete") {
                    debug!("delete");
                    if message.chat.kind == "private" {
                        debug!("private");
                        let tg = self.tg.clone();
                        let chat_id = message.chat.id;

                        Arbiter::handle().spawn(
                            self.db
                                .call_fut(LookupEventsByUserId { user_id: user.id })
                                .then(flatten::<LookupEventsByUserId>)
                                .then(move |events| match events {
                                    Ok(events) => Ok(tg.send(AskDeleteEvents(events, chat_id))),
                                    Err(e) => {
                                        tg.send(SendError(
                                            chat_id,
                                            "Failed to get events for user",
                                        ));
                                        Err(e)
                                    }
                                })
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
                } else if text.starts_with("/events") {
                    debug!("events");
                    if message.chat.kind == "group" || message.chat.kind == "supergroup" {
                        debug!("group | supergroup");
                        let tg = self.tg.clone();

                        let chat_id = message.chat.id;

                        Arbiter::handle().spawn(
                            self.db
                                .call_fut(LookupEventsByChatId { chat_id })
                                .then(flatten::<LookupEventsByChatId>)
                                .then(move |events| match events {
                                    Ok(events) => Ok(tg.send(SendEvents(chat_id, events))),
                                    Err(e) => {
                                        tg.send(SendError(chat_id, "Failed to fetch events"));
                                        Err(e)
                                    }
                                })
                                .map_err(|e| error!("Error: {:?}", e)),
                        )
                    }
                } else if text.starts_with("/help")
                    || (text.starts_with("/start") && message.chat.kind == "private")
                {
                    debug!("help | start + private");
                    self.tg.send(SendHelp(message.chat.id));
                } else {
                    debug!("else");
                    if message.chat.kind == "group" || message.chat.kind == "supergroup" {
                        debug!("group | supergroup");
                        let db = self.db.clone();

                        let user_id = user.id;
                        let username = user.username.unwrap_or(user.first_name);
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
                                            db.send(NewUser {
                                                chat_id,
                                                user_id,
                                                username,
                                            });
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
                if let Ok(query_data) = serde_json::from_str::<CallbackQueryMessage>(&data) {
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
                            match query_data {
                                CallbackQueryMessage::NewEvent { channel_id } => {
                                    debug!("channel_id: {}", channel_id);
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
                                                    })
                                            })
                                            .then(move |nel| match nel {
                                                Ok(nel) => Ok(tg.send(SendUrl(
                                                    chat_id,
                                                    "create".to_owned(),
                                                    format!(
                                                        "{}/events/new/{}={}",
                                                        url,
                                                        base64d,
                                                        nel.id()
                                                    ),
                                                ))),
                                                Err(e) => {
                                                    tg.send(SendError(
                                                        chat_id,
                                                        "Failed to generate new event link",
                                                    ));
                                                    Err(e)
                                                }
                                            })
                                            .map_err(|e| error!("Error: {:?}", e)),
                                    );
                                }
                                CallbackQueryMessage::EditEvent { event_id } => {
                                    Arbiter::handle().spawn(
                                        self.db
                                            .call_fut(LookupEvent { event_id })
                                            .then(flatten::<LookupEvent>)
                                            .and_then(move |event| {
                                                if event
                                                    .hosts()
                                                    .iter()
                                                    .any(|host| host.user_id() == user_id)
                                                {
                                                    Ok(event)
                                                } else {
                                                    Err(EventErrorKind::Lookup.into())
                                                }
                                            })
                                            .and_then(move |event| {
                                                let e2 = event.clone();
                                                let host = e2.hosts()
                                                    .iter()
                                                    .find(|host| host.user_id() == user_id)
                                                    .unwrap();

                                                db2.call_fut(StoreEditEventLink {
                                                    user_id: host.id(),
                                                    system_id: event.system_id(),
                                                    event_id: event.id(),
                                                    secret,
                                                }).then(flatten::<StoreEditEventLink>)
                                            })
                                            .then(move |eel| match eel {
                                                Ok(eel) => Ok(tg.send(SendUrl(
                                                    chat_id,
                                                    "update".to_owned(),
                                                    format!(
                                                        "{}/events/edit/{}={}",
                                                        url,
                                                        base64d,
                                                        eel.id()
                                                    ),
                                                ))),
                                                Err(e) => {
                                                    tg.send(SendError(
                                                        chat_id,
                                                        "Unable to generate edit link",
                                                    ));
                                                    Err(e)
                                                }
                                            })
                                            .map_err(|e| error!("Error: {:?}", e)),
                                    );
                                }
                                CallbackQueryMessage::DeleteEvent {
                                    event_id,
                                    system_id,
                                    title,
                                } => Arbiter::handle().spawn(
                                    self.db
                                        .call_fut(DeleteEvent { event_id })
                                        .then(flatten::<DeleteEvent>)
                                        .and_then(move |_| {
                                            db.call_fut(LookupSystem { system_id })
                                                .then(flatten::<LookupSystem>)
                                        })
                                        .then(move |chat_system| match chat_system {
                                            Ok(chat_system) => Ok(tg.send(EventDeleted(
                                                chat_id,
                                                chat_system.events_channel(),
                                                title,
                                            ))),
                                            Err(e) => {
                                                tg.send(SendError(
                                                    chat_id,
                                                    "Failed to delete event",
                                                ));
                                                Err(e)
                                            }
                                        })
                                        .map_err(|e| error!("Error: {:?}", e)),
                                ),
                            }
                        }
                    }
                }
            }
        }
    }
}
