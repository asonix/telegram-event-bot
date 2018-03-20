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

//! This module defines the `TelegramActor` struct and related functions. It handles talking to
//! Telegram.

use std::fmt::Debug;
use std::collections::HashSet;

use actix::{Addr, Arbiter, Syn, Unsync};
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
use actors::db_broker::messages::{DeleteEvent, DeleteUserByUserId, GetEventsForSystem,
                                  LookupEvent, LookupEventsByChatId, LookupEventsByUserId,
                                  LookupSystem, LookupSystemByChannel, LookupUser, NewChannel,
                                  NewChat, NewRelation, NewUser, RemoveUserChat,
                                  StoreEditEventLink, StoreEventLink};
use actors::db_broker::DbBroker;
use actors::users_actor::{DeleteState, UserState, UsersActor};
use actors::users_actor::messages::{LookupChannels, RemoveRelation, TouchChannel, TouchUser};
use error::{EventError, EventErrorKind};
use models::chat_system::ChatSystem;
use models::event::Event;
use util::flatten;

mod actor;
pub mod messages;

/// This type defines all the possible shapes of data coming from a Telegram Callback Query
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

/// Define the Telegram Actor. It knows the base URL of the Web UI, and can talk to the database,
/// the users actor, and Telegram itself.
pub struct TelegramActor {
    url: String,
    bot: RcBot,
    db: Addr<Unsync, DbBroker>,
    users: Addr<Syn, UsersActor>,
}

impl TelegramActor {
    pub fn new(
        url: String,
        bot: RcBot,
        db: Addr<Unsync, DbBroker>,
        users: Addr<Syn, UsersActor>,
    ) -> Self {
        TelegramActor {
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

                // Spawn a future that handles removing a user from a chat
                Arbiter::handle().spawn(
                    self.users
                        .send(RemoveRelation(user_id, chat_id))
                        .then(flatten)
                        .map(move |delete_state| {
                            match delete_state {
                                DeleteState::UserEmpty => Arbiter::handle().spawn(
                                    db.send(DeleteUserByUserId(user_id))
                                        .then(flatten)
                                        .map_err(|e| error!("Error deleting User: {:?}", e)),
                                ),
                                _ => (),
                            }

                            Arbiter::handle().spawn(
                                db.send(RemoveUserChat(user_id, chat_id))
                                    .then(flatten)
                                    .map_err(|e| error!("Error removing UserChat: {:?}", e)),
                            );
                        })
                        .map_err(|e| error!("Error removing User/Chat relation: {:?}", e)),
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

                // Spawn a future that handles adding a user to a chat
                Arbiter::handle().spawn(
                    self.users
                        .send(TouchUser(user_id, chat_id))
                        .then(flatten)
                        .map(move |user_state| match user_state {
                            UserState::NewRelation => {
                                debug!("Sending NewRelation");
                                db.do_send(NewRelation { chat_id, user_id });
                            }
                            UserState::NewUser => {
                                debug!("Sending NewUser");
                                db.do_send(NewUser {
                                    chat_id,
                                    user_id,
                                    username,
                                });
                            }
                            _ => (),
                        })
                        .map_err(|e| error!("Error touching user/chat relation: {:?}", e)),
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

                        // spawn a future that handles asking the User which chat they want to
                        // create an event for
                        Arbiter::handle().spawn(
                            self.users
                                .send(LookupChannels(user.id))
                                .then(flatten)
                                .then(move |chats| match chats {
                                    Ok(chats) => Ok(TelegramActor::ask_chats(bot, chats, chat_id)),
                                    Err(e) => {
                                        TelegramActor::send_error(
                                            &bot,
                                            chat_id,
                                            "Failed to get event channnels for user",
                                        );
                                        Err(e)
                                    }
                                })
                                .map_err(|e| error!("Error looking up channel: {:?}", e)),
                        );
                    } else {
                        debug!("not private");
                        self.notify_private(message.chat.id);
                    }
                } else if text.starts_with("/edit") {
                    debug!("edit");
                    if message.chat.kind == "private" {
                        debug!("private");
                        let bot = self.bot.clone();
                        let chat_id = message.chat.id;

                        // spawn a future that handles asking the User which event they would like
                        // to edit.
                        //
                        // Users can only edit events they host
                        Arbiter::handle().spawn(
                            self.db
                                .send(LookupEventsByUserId { user_id: user.id })
                                .then(flatten)
                                .then(move |events| match events {
                                    Ok(events) => {
                                        Ok(TelegramActor::ask_events(bot, events, chat_id))
                                    }
                                    Err(e) => {
                                        TelegramActor::send_error(
                                            &bot,
                                            chat_id,
                                            "Failed to get events for user",
                                        );
                                        Err(e)
                                    }
                                })
                                .map_err(|e| error!("Error looking up events: {:?}", e)),
                        );
                    } else {
                        debug!("not private");
                        self.notify_private(message.chat.id);
                    }
                } else if text.starts_with("/delete") {
                    debug!("delete");
                    if message.chat.kind == "private" {
                        debug!("private");
                        let bot = self.bot.clone();
                        let chat_id = message.chat.id;

                        // Spawn a future that handles asking the user which event they would like
                        // to delete.
                        //
                        // Users can only delete events they host.
                        Arbiter::handle().spawn(
                            self.db
                                .send(LookupEventsByUserId { user_id: user.id })
                                .then(flatten)
                                .then(move |events| match events {
                                    Ok(events) => {
                                        Ok(TelegramActor::ask_delete_events(bot, events, chat_id))
                                    }
                                    Err(e) => {
                                        TelegramActor::send_error(
                                            &bot,
                                            chat_id,
                                            "Failed to get events for user",
                                        );
                                        Err(e)
                                    }
                                })
                                .map_err(|e| error!("Error looking up events: {:?}", e)),
                        );
                    } else {
                        debug!("not private");
                        self.notify_private(message.chat.id);
                    }
                } else if text.starts_with("/id") {
                    debug!("id");
                    let chat_id = message.chat.id;

                    if message.chat.kind == "group" || message.chat.kind == "supergroup" {
                        debug!("group | supergroup");

                        // Print the ID of the given chat
                        TelegramActor::print_id(&self.bot, chat_id);
                    } else {
                        TelegramActor::send_error(&self.bot, chat_id, "Cannot link non-group chat");
                    }
                } else if text.starts_with("/events") {
                    debug!("events");
                    let chat_id = message.chat.id;

                    if message.chat.kind == "group" || message.chat.kind == "supergroup" {
                        debug!("group | supergroup");
                        let bot = self.bot.clone();

                        // Spawn a future that handles printing the events for a given chat
                        Arbiter::handle().spawn(
                            self.db
                                .send(LookupEventsByChatId { chat_id })
                                .then(flatten)
                                .then(move |events| match events {
                                    Ok(events) => {
                                        Ok(TelegramActor::send_events(&bot, chat_id, events))
                                    }
                                    Err(e) => {
                                        TelegramActor::send_error(
                                            &bot,
                                            chat_id,
                                            "Failed to fetch events",
                                        );
                                        Err(e)
                                    }
                                })
                                .map_err(|e| error!("Error looking up events: {:?}", e)),
                        )
                    } else {
                        TelegramActor::send_error(&self.bot, chat_id, "Could not fetch events");
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

                        // Spawn a future that handles updating a user/chat relation
                        Arbiter::handle().spawn(
                            self.users
                                .send(TouchUser(user_id, chat_id))
                                .then(flatten)
                                .and_then(move |user_state| {
                                    Ok(match user_state {
                                        UserState::NewRelation => {
                                            debug!("Sending NewRelation");
                                            db.do_send(NewRelation { chat_id, user_id });
                                        }
                                        UserState::NewUser => {
                                            debug!("Sending NewUser");
                                            db.do_send(NewUser {
                                                chat_id,
                                                user_id,
                                                username,
                                            });
                                        }
                                        _ => (),
                                    })
                                })
                                .map_err(|e| error!("Error Updating user/chat relations: {:?}", e)),
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

                    // Get the valid IDs provided in the link message, update the UserActor with
                    // the valid links
                    let chat_ids = text.trim_left_matches("/link")
                        .split(' ')
                        .into_iter()
                        .filter_map(|chat_id| chat_id.parse::<Integer>().ok())
                        .map(|chat_id| {
                            self.users.do_send(TouchChannel(channel_id, chat_id));

                            chat_id
                        })
                        .collect();

                    // Spawn a future updating the links between the channel and the given chats in
                    // the database
                    Arbiter::handle().spawn(
                        self.is_admin(channel_id, chat_ids)
                            .then(move |res| match res {
                                Ok(item) => Ok((item, bot)),
                                Err(err) => Err((err, bot)),
                            })
                            .and_then(move |(chat_ids, bot)| {
                                for chat_id in chat_ids.iter() {
                                    db.do_send(NewChat {
                                        channel_id: channel_id,
                                        chat_id: *chat_id,
                                    });
                                }

                                TelegramActor::linked(&bot, channel_id, chat_ids);
                                Ok(())
                            })
                            .map_err(move |(e, bot)| {
                                TelegramActor::send_error(
                                    &bot,
                                    channel_id,
                                    "Could not determine if you are an admin of provided chats",
                                );
                                e
                            })
                            .map_err(|e| error!("Error checking admin: {:?}", e)),
                    );
                }
            } else if text.starts_with("/init") {
                debug!("init");
                if message.chat.kind == "channel" {
                    debug!("channel");
                    let channel_id = message.chat.id;
                    let bot = self.bot.clone();

                    // Spawn a future that adds the given channel to the database
                    Arbiter::handle().spawn(
                        self.db
                            .send(NewChannel { channel_id })
                            .then(flatten)
                            .then(move |res| match res {
                                Ok(item) => Ok((item, bot)),
                                Err(err) => Err((err, bot)),
                            })
                            .map(move |(_chat_system, bot)| {
                                TelegramActor::created_channel(&bot, channel_id)
                            })
                            .map_err(move |(e, bot)| {
                                TelegramActor::send_error(
                                    &bot,
                                    channel_id,
                                    "Could not initialize the chat",
                                );
                                e
                            })
                            .map_err(|e| error!("Error creating channel: {:?}", e)),
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
                                    // Spawn a future that creates a new event
                                    debug!("channel_id: {}", channel_id);
                                    Arbiter::handle().spawn(
                                        self.db
                                            .send(LookupUser(user_id))
                                            .then(flatten)
                                            .and_then(move |user| {
                                                db.send(LookupSystemByChannel(channel_id))
                                                    .then(flatten)
                                                    .map(|chat_system| (chat_system, user))
                                            })
                                            .and_then(move |(chat_system, user)| {
                                                let events_channel = chat_system.events_channel();
                                                users
                                                    .send(LookupChannels(user.user_id()))
                                                    .then(flatten)
                                                    .and_then(move |channel_ids| {
                                                        if channel_ids.contains(&events_channel) {
                                                            Ok(())
                                                        } else {
                                                            Err(EventErrorKind::Permissions.into())
                                                        }
                                                    })
                                                    .and_then(move |_| {
                                                        db2.send(StoreEventLink {
                                                            user_id: user.id(),
                                                            system_id: chat_system.id(),
                                                            secret,
                                                        }).then(flatten)
                                                    })
                                            })
                                            .then(move |nel| match nel {
                                                Ok(nel) => Ok(TelegramActor::send_url(
                                                    &bot,
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
                                                    TelegramActor::send_error(
                                                        &bot,
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
                                    // Spawn a future that updates a given event
                                    Arbiter::handle().spawn(
                                        self.db
                                            .send(LookupEvent { event_id })
                                            .then(flatten)
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

                                                db2.send(StoreEditEventLink {
                                                    user_id: host.id(),
                                                    system_id: event.system_id(),
                                                    event_id: event.id(),
                                                    secret,
                                                }).then(flatten)
                                            })
                                            .then(move |eel| match eel {
                                                Ok(eel) => Ok(TelegramActor::send_url(
                                                    &bot,
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
                                                    TelegramActor::send_error(
                                                        &bot,
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
                                    // Spawn a future taht deletes the given event
                                    self.db
                                        .send(DeleteEvent { event_id })
                                        .then(flatten)
                                        .and_then(move |_| {
                                            db.send(LookupSystem { system_id }).then(flatten)
                                        })
                                        .then(move |chat_system| match chat_system {
                                            Ok(chat_system) => Ok(TelegramActor::event_deleted(
                                                &bot,
                                                chat_id,
                                                chat_system.events_channel(),
                                                title,
                                            )),
                                            Err(e) => {
                                                TelegramActor::send_error(
                                                    &bot,
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

    fn event_soon(&self, event: Event) {
        let bot = self.bot.clone();

        let fut = self.db
            .send(LookupSystem {
                system_id: event.system_id(),
            })
            .then(flatten)
            .and_then(move |chat_system| {
                bot.message(
                    chat_system.events_channel(),
                    format!("Don't forget! {} is starting soon!", event.title()),
                ).send()
                    .map_err(|e| e.context(EventErrorKind::Telegram).into())
            })
            .map(|_| ())
            .map_err(|e| error!("Error: {:?}", e));

        self.bot.inner.handle.spawn(fut);
    }

    fn event_over(&self, event: Event) {
        let bot = self.bot.clone();

        let id = event.id();
        let system_id = event.system_id();

        let fut = self.db
            .send(LookupSystem { system_id })
            .then(flatten)
            .and_then(move |chat_system| {
                bot.message(
                    chat_system.events_channel(),
                    format!("{} has ended!", event.title()),
                ).send()
                    .map_err(|e| e.context(EventErrorKind::Telegram).into())
            })
            .map(|_| ())
            .map_err(|e| error!("Error: {:?}", e));

        self.bot.inner.handle.spawn(fut);

        self.query_events(id, system_id);
    }

    fn event_started(&self, event: Event) {
        let bot = self.bot.clone();

        let fut = self.db
            .send(LookupSystem {
                system_id: event.system_id(),
            })
            .then(flatten)
            .and_then(move |chat_system| {
                bot.message(
                    chat_system.events_channel(),
                    format!("{} has started!", event.title()),
                ).send()
                    .map_err(|e| e.context(EventErrorKind::Telegram).into())
            })
            .map(|_| ())
            .map_err(|e| error!("Error: {:?}", e));

        self.bot.inner.handle.spawn(fut);
    }

    fn new_event(&self, event: Event) {
        let localtime = event.start_date().with_timezone(&Central);
        let when = format_date(localtime);
        let hosts = event
            .hosts()
            .iter()
            .map(|host| format!("@{}", host.username()))
            .collect::<Vec<_>>()
            .join(", ");

        let length = format_duration(&event);

        let bot = self.bot.clone();

        let fut = self.db
            .send(LookupSystem {
                system_id: event.system_id(),
            })
            .then(flatten)
            .and_then(move |chat_system| {
                bot.message(
                    chat_system.events_channel(),
                    format!(
                        "New Event!\n{}\nWhen: {}\nDuration: {}\nDescription: {}\nHosts: {}",
                        event.title(),
                        when,
                        length,
                        event.description(),
                        hosts
                    ),
                ).send()
                    .map_err(|e| e.context(EventErrorKind::Telegram).into())
            })
            .map(|_| ())
            .map_err(|e| error!("Error: {:?}", e));

        self.bot.inner.handle.spawn(fut);
    }

    fn update_event(&self, event: Event) {
        let localtime = event.start_date().with_timezone(&Central);
        let when = format_date(localtime);

        let length = format_duration(&event);

        let bot = self.bot.clone();

        let fut = self.db
            .send(LookupSystem {
                system_id: event.system_id(),
            })
            .then(flatten)
            .and_then(move |chat_system| {
                bot.message(
                    chat_system.events_channel(),
                    format!(
                        "Event Updated!\n{}\nWhen: {}\nDuration: {}\nDescription: {}",
                        event.title(),
                        when,
                        length,
                        event.description(),
                    ),
                ).send()
                    .map_err(|e| e.context(EventErrorKind::Telegram).into())
            })
            .map(|_| ())
            .map_err(|e| error!("Error: {:?}", e));

        self.bot.inner.handle.spawn(fut);
    }

    fn query_events(&self, event_id: i32, system_id: i32) {
        let db = self.db.clone();
        let bot = self.bot.clone();

        let fut = self.db
            .send(LookupSystem { system_id })
            .then(flatten)
            .map_err(|e| {
                error!("LookupSystem");
                e
            })
            .and_then(move |chat_system: ChatSystem| {
                db.send(GetEventsForSystem { system_id })
                    .then(flatten)
                    .map_err(|e| {
                        error!("GetEventsForSystem");
                        e
                    })
                    .and_then(move |events: Vec<Event>| {
                        let events = events
                            .into_iter()
                            .filter(|event| event.id() != event_id)
                            .collect();

                        print_events(&bot, chat_system.events_channel(), events)
                    })
            });

        self.bot
            .inner
            .handle
            .spawn(fut.map(|_| ()).map_err(|e| error!("Error: {:?}", e)));
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
                let msg = if buttons.len() > 0 {
                    bot2.message(
                        chat_id,
                        "Which channel would you like to create an event for?".to_owned(),
                    ).reply_markup(InlineKeyboardMarkup::new(vec![buttons]))
                } else {
                    bot2.message(chat_id, "You aren't in any chats with an associated events channel. If you believe this a mistake, please send a message in the associated chat first, then try again".to_owned())
                };

                msg.send()
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
                let msg = if buttons.len() > 0 {
                    bot2.message(chat_id, "Which event would you like to delete?".to_owned())
                        .reply_markup(InlineKeyboardMarkup::new(vec![buttons]))
                } else {
                    bot2.message(chat_id, "You aren't hosting any events".to_owned())
                };
                msg.send()
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
                let msg = if buttons.len() > 0 {
                    bot2.message(chat_id, "Which event would you like to edit?".to_owned())
                        .reply_markup(InlineKeyboardMarkup::new(vec![buttons]))
                } else {
                    bot2.message(chat_id, "You aren't hosting any events".to_owned())
                };
                msg.send()
                    .map_err(|e| EventError::from(e.context(EventErrorKind::Telegram)))
            });

        bot.inner
            .handle
            .spawn(fut.map(|_| ()).map_err(|e| error!("Error: {:?}", e)));
    }

    fn event_deleted(bot: &RcBot, chat_id: Integer, channel_id: Integer, title: String) {
        send_message(bot, chat_id, "Deleted event!".to_owned());

        send_message(bot, channel_id, format!("Event deleted: {}", title));
    }

    fn notify_private(&self, chat_id: Integer) {
        send_message(
            &self.bot,
            chat_id,
            "Please send this command as a private message".to_owned(),
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
        send_message(
            &self.bot,
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
        );
    }

    fn send_error(bot: &RcBot, chat_id: Integer, error: &str) {
        send_message(bot, chat_id, error.to_owned());
    }

    fn send_url(bot: &RcBot, chat_id: Integer, action: String, url: String) {
        send_message(
            bot,
            chat_id,
            format!("Use this link to {} your event: {}", action, url),
        );
    }

    fn send_events(bot: &RcBot, chat_id: Integer, events: Vec<Event>) {
        bot.inner.handle.spawn(
            print_events(bot, chat_id, events)
                .map_err(|e| error!("Error sending events to Telegram: {:?}", e)),
        );
    }

    fn print_id(bot: &RcBot, chat_id: Integer) {
        send_message(bot, chat_id, format!("{}", chat_id));
    }

    fn linked(bot: &RcBot, channel_id: Integer, chat_ids: Vec<Integer>) {
        let msg = format!(
            "Linked channel '{}' to chats ({})",
            channel_id,
            chat_ids
                .into_iter()
                .map(|id| format!("{}", id))
                .collect::<Vec<_>>()
                .join(", ")
        );

        send_message(bot, channel_id, msg);
    }

    fn created_channel(bot: &RcBot, channel_id: Integer) {
        send_message(bot, channel_id, "Initialized".to_owned());
    }
}

fn send_message(bot: &RcBot, chat_id: Integer, message: String) {
    bot.inner.handle.spawn(
        bot.message(chat_id, message)
            .send()
            .map(|_| ())
            .map_err(|e| error!("Error sending message to Telegram: {:?}", e)),
    );
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
    bot: &RcBot,
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
                "----Event----\n{}\nWhen: {}\nDuration: {}\nDescription: {}\nHosts: {}",
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
