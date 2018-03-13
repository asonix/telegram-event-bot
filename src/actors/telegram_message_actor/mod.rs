use std::fmt::Debug;
use std::collections::HashSet;

use actix::{Address, Arbiter};
use chrono::{DateTime, Datelike, TimeZone, Timelike, Weekday};
use chrono_tz::US::Central;
use futures::{Future, Stream};
use futures::stream::iter_ok;
use telebot::objects::{CallbackQuery, InlineKeyboardButton, InlineKeyboardMarkup, Integer,
                       Message, Update};
use telebot::RcBot;
use base_x::encode;
use event_web::generate_secret;
use rand::Rng;
use rand::os::OsRng;
use serde_json;
use telebot::functions::{FunctionGetChat, FunctionGetChatAdministrators, FunctionMessage};

use ENCODING_ALPHABET;
use actors::db_broker::messages::{DeleteEvent, DeleteUserByUserId, LookupEvent,
                                  LookupEventsByChatId, LookupEventsByUserId, LookupSystem,
                                  LookupSystemByChannel, LookupUser, NewChannel, NewChat,
                                  NewRelation, NewUser, RemoveUserChat, StoreEditEventLink,
                                  StoreEventLink};
use actors::db_broker::DbBroker;
use actors::users_actor::{DeleteState, UserState, UsersActor};
use actors::users_actor::messages::{LookupChannels, RemoveRelation, TouchChannel, TouchUser};
use error::{EventError, EventErrorKind};
use models::event::Event;
use util::flatten;

mod actor;
mod messages;

pub use self::messages::StartStreaming;

#[derive(Clone, Debug, Deserialize, Serialize)]
pub enum CallbackQueryMessage {
    NewEvent {
        channel_id: Integer,
    },
    EditEvent {
        event_id: i32,
    },
    DeleteEvent {
        event_id: i32,
        system_id: i32,
        title: String,
    },
}

pub struct TelegramMessageActor {
    url: String,
    bot: RcBot,
    db: Address<DbBroker>,
    users: Address<UsersActor>,
}

impl TelegramMessageActor {
    pub fn new(url: String, bot: RcBot, db: Address<DbBroker>, users: Address<UsersActor>) -> Self {
        TelegramMessageActor {
            url,
            bot,
            db,
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
                        let bot = self.bot.clone();
                        let chat_id = message.chat.id;

                        Arbiter::handle().spawn(
                            self.users
                                .call_fut(LookupChannels(user.id))
                                .then(flatten::<LookupChannels>)
                                .then(move |chats| match chats {
                                    Ok(chats) => {
                                        Ok(TelegramMessageActor::ask_chats(bot, chats, chat_id))
                                    }
                                    Err(e) => {
                                        TelegramMessageActor::send_error(
                                            bot,
                                            chat_id,
                                            "Failed to get event channnels for user",
                                        );
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
                        let bot = self.bot.clone();
                        let chat_id = message.chat.id;

                        Arbiter::handle().spawn(
                            self.db
                                .call_fut(LookupEventsByUserId { user_id: user.id })
                                .then(flatten::<LookupEventsByUserId>)
                                .then(move |events| match events {
                                    Ok(events) => {
                                        Ok(TelegramMessageActor::ask_events(bot, events, chat_id))
                                    }
                                    Err(e) => {
                                        TelegramMessageActor::send_error(
                                            bot,
                                            chat_id,
                                            "Failed to get events for user",
                                        );
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
                        let bot = self.bot.clone();
                        let chat_id = message.chat.id;

                        Arbiter::handle().spawn(
                            self.db
                                .call_fut(LookupEventsByUserId { user_id: user.id })
                                .then(flatten::<LookupEventsByUserId>)
                                .then(move |events| match events {
                                    Ok(events) => Ok(TelegramMessageActor::ask_delete_events(
                                        bot,
                                        events,
                                        chat_id,
                                    )),
                                    Err(e) => {
                                        TelegramMessageActor::send_error(
                                            bot,
                                            chat_id,
                                            "Failed to get events for user",
                                        );
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

                        TelegramMessageActor::print_id(self.bot.clone(), chat_id);
                    }
                } else if text.starts_with("/events") {
                    debug!("events");
                    if message.chat.kind == "group" || message.chat.kind == "supergroup" {
                        debug!("group | supergroup");
                        let bot = self.bot.clone();
                        let chat_id = message.chat.id;

                        Arbiter::handle().spawn(
                            self.db
                                .call_fut(LookupEventsByChatId { chat_id })
                                .then(flatten::<LookupEventsByChatId>)
                                .then(move |events| match events {
                                    Ok(events) => {
                                        Ok(TelegramMessageActor::send_events(bot, chat_id, events))
                                    }
                                    Err(e) => {
                                        TelegramMessageActor::send_error(
                                            bot,
                                            chat_id,
                                            "Failed to fetch events",
                                        );
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
                    self.send_help(message.chat.id);
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
                    let bot = self.bot.clone();
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
                        self.is_admin(channel_id, chat_ids)
                            .and_then(move |chat_ids| {
                                for chat_id in chat_ids.iter() {
                                    db.send(NewChat {
                                        channel_id: channel_id,
                                        chat_id: *chat_id,
                                    });
                                }

                                TelegramMessageActor::linked(bot, channel_id, chat_ids);
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
                    let bot = self.bot.clone();

                    Arbiter::handle().spawn(
                        self.db
                            .call_fut(NewChannel { channel_id })
                            .then(flatten::<NewChannel>)
                            .map(move |_chat_system| {
                                TelegramMessageActor::created_channel(bot, channel_id)
                            })
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
                            let bot = self.bot.clone();
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
                                                Ok(nel) => Ok(TelegramMessageActor::send_url(
                                                    bot,
                                                    chat_id,
                                                    "create".to_owned(),
                                                    format!(
                                                        "{}/events/new/{}={}",
                                                        url,
                                                        base64d,
                                                        nel.id()
                                                    ),
                                                )),
                                                Err(e) => {
                                                    TelegramMessageActor::send_error(
                                                        bot,
                                                        chat_id,
                                                        "Failed to generate new event link",
                                                    );
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
                                                Ok(eel) => Ok(TelegramMessageActor::send_url(
                                                    bot,
                                                    chat_id,
                                                    "update".to_owned(),
                                                    format!(
                                                        "{}/events/edit/{}={}",
                                                        url,
                                                        base64d,
                                                        eel.id()
                                                    ),
                                                )),
                                                Err(e) => {
                                                    TelegramMessageActor::send_error(
                                                        bot,
                                                        chat_id,
                                                        "Unable to generate edit link",
                                                    );
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
                                            Ok(chat_system) => {
                                                Ok(TelegramMessageActor::event_deleted(
                                                    bot,
                                                    chat_id,
                                                    chat_system.events_channel(),
                                                    title,
                                                ))
                                            }
                                            Err(e) => {
                                                TelegramMessageActor::send_error(
                                                    bot,
                                                    chat_id,
                                                    "Failed to delete event",
                                                );
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

    fn ask_chats(bot: RcBot, channels: HashSet<Integer>, chat_id: Integer) {
        let bot2 = bot.clone();
        let bot3 = bot.clone();

        let fut = iter_ok(channels)
            .and_then(move |channel_id| {
                bot.clone()
                    .get_chat(channel_id)
                    .send()
                    .map_err(|e| e.context(EventErrorKind::TelegramLookup).into())
            })
            .map(move |(_, channel)| {
                debug!("Asking about channel_id: {}", channel.id);
                InlineKeyboardButton::new(
                    channel
                        .title
                        .unwrap_or(channel.username.unwrap_or("No title".to_owned())),
                ).callback_data(
                    serde_json::to_string(&CallbackQueryMessage::NewEvent {
                        channel_id: channel.id,
                    }).unwrap(),
                )
            })
            .collect()
            .and_then(move |buttons| {
                bot2.message(
                    chat_id,
                    "Which channel would you like to create an event for?".to_owned(),
                ).reply_markup(InlineKeyboardMarkup::new(vec![buttons]))
                    .send()
                    .map_err(|e| EventError::from(e.context(EventErrorKind::Telegram)))
            });

        bot3.inner
            .handle
            .spawn(fut.map(|_| ()).map_err(|e| error!("Error: {:?}", e)));
    }

    fn ask_delete_events(bot: RcBot, events: Vec<Event>, chat_id: Integer) {
        let bot2 = bot.clone();

        let fut = iter_ok(events)
            .map(|event| {
                InlineKeyboardButton::new(event.title().to_owned()).callback_data(
                    serde_json::to_string(&CallbackQueryMessage::DeleteEvent {
                        event_id: event.id(),
                        system_id: event.system_id(),
                        title: event.title().to_owned(),
                    }).unwrap(),
                )
            })
            .collect()
            .and_then(move |buttons| {
                bot2.message(chat_id, "Which event would you like to delete?".to_owned())
                    .reply_markup(InlineKeyboardMarkup::new(vec![buttons]))
                    .send()
                    .map_err(|e| EventError::from(e.context(EventErrorKind::Telegram)))
            });

        bot.inner
            .handle
            .spawn(fut.map(|_| ()).map_err(|e| error!("Error: {:?}", e)));
    }

    fn ask_events(bot: RcBot, events: Vec<Event>, chat_id: Integer) {
        let bot2 = bot.clone();

        let fut = iter_ok(events)
            .map(|event| {
                InlineKeyboardButton::new(event.title().to_owned()).callback_data(
                    serde_json::to_string(&CallbackQueryMessage::EditEvent {
                        event_id: event.id(),
                    }).unwrap(),
                )
            })
            .collect()
            .and_then(move |buttons| {
                bot2.message(chat_id, "Which event would you like to edit?".to_owned())
                    .reply_markup(InlineKeyboardMarkup::new(vec![buttons]))
                    .send()
                    .map_err(|e| EventError::from(e.context(EventErrorKind::Telegram)))
            });

        bot.inner
            .handle
            .spawn(fut.map(|_| ()).map_err(|e| error!("Error: {:?}", e)));
    }

    fn event_deleted(bot: RcBot, chat_id: Integer, channel_id: Integer, title: String) {
        bot.inner.handle.spawn(
            bot.message(chat_id, "Deleted event!".to_owned())
                .send()
                .map(|_| ())
                .map_err(|e| error!("Error: {:?}", e)),
        );

        bot.inner.handle.spawn(
            bot.message(channel_id, format!("Event deleted: {}", title))
                .send()
                .map(|_| ())
                .map_err(|e| error!("Error: {:?}", e)),
        );
    }

    fn is_admin(
        &self,
        channel_id: Integer,
        chat_ids: Vec<Integer>,
    ) -> impl Future<Item = Vec<Integer>, Error = EventError> {
        self.bot
            .unban_chat_administrators(channel_id)
            .send()
            .map_err(|e| EventError::from(e.context(EventErrorKind::TelegramLookup)))
            .and_then(move |(bot, admins)| {
                let channel_admins = admins
                    .into_iter()
                    .map(|admin| admin.user.id)
                    .collect::<HashSet<_>>();

                iter_ok(chat_ids)
                    .and_then(move |chat_id| {
                        bot.unban_chat_administrators(chat_id)
                            .send()
                            .map_err(|e| e.context(EventErrorKind::TelegramLookup).into())
                            .map(move |(bot, admins)| (bot, admins, chat_id))
                    })
                    .filter_map(move |(_, admins, chat_id)| {
                        if admins
                            .into_iter()
                            .any(|admin| channel_admins.contains(&admin.user.id))
                        {
                            Some(chat_id)
                        } else {
                            None
                        }
                    })
                    .collect()
            })
    }

    fn send_help(&self, chat_id: Integer) {
        self.bot.inner.handle.spawn(
            self.bot
                .message(
                    chat_id,
                    "/init - Initialize an event channel
/link - link a group chat with an event channel (usage: /link [chat_id])
/id - get the id of a group chat
/events - get a list of events for the current chat
/new - Create a new event (in a private chat with the bot)
/edit - Edit an event you're hosting (in a private chat with the bot)
/delete - Delete an event you're hosting (in a private chat with the bot)
/help - Print this help message"
                        .to_owned(),
                )
                .send()
                .map(|_| ())
                .map_err(|e| error!("Error: {:?}", e)),
        );
    }

    fn send_error(bot: RcBot, chat_id: Integer, error: &str) {
        bot.inner.handle.spawn(
            bot.message(chat_id, error.to_owned())
                .send()
                .map(|_| ())
                .map_err(|e| error!("Error: {:?}", e)),
        );
    }

    fn send_url(bot: RcBot, chat_id: Integer, action: String, url: String) {
        bot.inner.handle.spawn(
            bot.message(
                chat_id,
                format!("Use this link to {} your event: {}", action, url),
            ).send()
                .map(|_| ())
                .map_err(|e| error!("Error: {:?}", e)),
        )
    }

    fn send_events(bot: RcBot, chat_id: Integer, events: Vec<Event>) {
        bot.inner.handle.spawn(
            print_events(bot.clone(), chat_id, events).map_err(|e| error!("Error: {:?}", e)),
        );
    }

    fn print_id(bot: RcBot, chat_id: Integer) {
        bot.inner.handle.spawn(
            bot.message(chat_id, format!("{}", chat_id))
                .send()
                .map(|_| ())
                .map_err(|e| error!("Error: {:?}", e)),
        );
    }

    fn linked(bot: RcBot, channel_id: Integer, chat_ids: Vec<Integer>) {
        let msg = format!(
            "Linked channel '{}' to chats ({})",
            channel_id,
            chat_ids
                .into_iter()
                .map(|id| format!("{}", id))
                .collect::<Vec<_>>()
                .join(", ")
        );

        bot.inner.handle.spawn(
            bot.message(channel_id, msg)
                .send()
                .map(|_| ())
                .map_err(|e| error!("Error: {:?}", e)),
        );
    }

    fn created_channel(bot: RcBot, channel_id: Integer) {
        bot.inner.handle.spawn(
            bot.message(channel_id, format!("Initialized"))
                .send()
                .map(|_| ())
                .map_err(|e| error!("Error: {:?}", e)),
        );
    }
}

fn format_duration(event: &Event) -> String {
    let duration = event
        .end_date()
        .signed_duration_since(event.start_date().clone());

    if duration.num_weeks() > 0 {
        format!("{} Weeks", duration.num_weeks())
    } else if duration.num_days() > 0 {
        format!("{} Days", duration.num_days())
    } else if duration.num_hours() > 0 {
        format!("{} Hours", duration.num_hours())
    } else if duration.num_minutes() > 0 {
        format!("{} Minutes", duration.num_minutes())
    } else {
        "No time".to_owned()
    }
}

fn print_events(
    bot: RcBot,
    chat_id: Integer,
    events: Vec<Event>,
) -> impl Future<Item = (), Error = EventError> {
    let events = events
        .into_iter()
        .map(|event| {
            let localtime = event.start_date().with_timezone(&Central);
            let when = format_date(localtime);
            let duration = format_duration(&event);
            let hosts = event
                .hosts()
                .iter()
                .map(|host| format!("@{}", host.username()))
                .collect::<Vec<_>>()
                .join(", ");

            format!(
                "{}\nWhen: {}\nDuration: {}\nDescription: {}\nHosts: {}",
                event.title(),
                when,
                duration,
                event.description(),
                hosts
            )
        })
        .collect::<Vec<_>>()
        .join("\n\n");

    let msg = if events.len() > 0 {
        format!("Upcoming Events:\n\n{}", events)
    } else {
        "No upcoming events".to_owned()
    };

    bot.message(chat_id, msg)
        .send()
        .map(|_| ())
        .map_err(|e| e.context(EventErrorKind::Telegram).into())
}

fn format_date<T>(localtime: DateTime<T>) -> String
where
    T: TimeZone + Debug,
{
    let weekday = match localtime.weekday() {
        Weekday::Mon => "Monday",
        Weekday::Tue => "Tuesday",
        Weekday::Wed => "Wednesday",
        Weekday::Thu => "Thursday",
        Weekday::Fri => "Friday",
        Weekday::Sat => "Saturday",
        Weekday::Sun => "Sunday",
    };

    let month = match localtime.month() {
        1 => "January",
        2 => "February",
        3 => "March",
        4 => "April",
        5 => "May",
        6 => "June",
        7 => "July",
        8 => "August",
        9 => "September",
        10 => "October",
        11 => "November",
        12 => "December",
        _ => "Unknown Month",
    };

    let day = match localtime.day() {
        1 | 21 | 31 => "st",
        2 | 22 => "nd",
        3 | 23 => "rd",
        _ => "th",
    };

    let minute = if localtime.minute() > 9 {
        format!("{}", localtime.minute())
    } else {
        format!("0{}", localtime.minute())
    };

    format!(
        "{}:{} {:?}, {}, {} {}{}",
        localtime.hour(),
        minute,
        localtime.timezone(),
        weekday,
        month,
        localtime.day(),
        day
    )
}
