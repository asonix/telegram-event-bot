use std::collections::HashSet;

use actix::{Actor, AsyncContext, Context, Handler};
use failure::Fail;
use futures::{Future, Stream};
use futures::stream::iter_ok;
use telebot::objects::Integer;

use error::{EventError, EventErrorKind};
use actors::db_actor::messages::{GetSystemsWithChats, GetUsersWithChats};
use models::user::User;
use models::chat::Chat;
use models::chat_system::ChatSystem;
use super::{UserState, UsersActor};
use super::messages::*;

impl Actor for UsersActor {
    type Context = Context<Self>;

    fn started(&mut self, ctx: &mut Self::Context) {
        let db = self.db.clone();

        ctx.add_stream(
            db.call_fut(GetUsersWithChats)
                .then(|msg_res| match msg_res {
                    Ok(res) => res,
                    Err(e) => Err(e.context(EventErrorKind::Canceled).into()),
                })
                .into_stream()
                .and_then(|users_with_chats: Vec<(User, Chat)>| {
                    Ok(iter_ok(
                        users_with_chats
                            .into_iter()
                            .map(|(u, c)| TouchUser(u.user_id(), c.chat_id())),
                    ))
                })
                .flatten(),
        );

        let db = self.db.clone();

        ctx.add_stream(
            db.call_fut(GetSystemsWithChats)
                .then(|msg_res| match msg_res {
                    Ok(res) => res,
                    Err(e) => Err(e.context(EventErrorKind::Canceled).into()),
                })
                .into_stream()
                .and_then(|systems_with_chats: Vec<(ChatSystem, Chat)>| {
                    Ok(iter_ok(systems_with_chats.into_iter().map(|(s, c)| {
                        TouchChannel(s.events_channel(), c.chat_id())
                    })))
                })
                .flatten(),
        );
    }
}

impl Handler<Result<TouchUser, EventError>> for UsersActor {
    type Result = Result<UserState, ()>;

    fn handle(
        &mut self,
        msg: Result<TouchUser, EventError>,
        _: &mut Self::Context,
    ) -> Self::Result {
        msg.map(|msg| self.touch_user(msg.0, msg.1))
            .map_err(|e| error!("Error: {:?}", e))
    }
}

impl Handler<Result<TouchChannel, EventError>> for UsersActor {
    type Result = ();

    fn handle(
        &mut self,
        msg: Result<TouchChannel, EventError>,
        _: &mut Self::Context,
    ) -> Self::Result {
        let _ = msg.map(|msg| self.touch_channel(msg.0, msg.1))
            .map_err(|e| error!("Error: {:?}", e));
    }
}

impl Handler<TouchUser> for UsersActor {
    type Result = Result<UserState, EventError>;

    fn handle(&mut self, msg: TouchUser, _: &mut Self::Context) -> Self::Result {
        Ok(self.touch_user(msg.0, msg.1))
    }
}

impl Handler<TouchChannel> for UsersActor {
    type Result = ();

    fn handle(&mut self, msg: TouchChannel, _: &mut Self::Context) -> Self::Result {
        self.touch_channel(msg.0, msg.1)
    }
}

impl Handler<LookupChats> for UsersActor {
    type Result = Result<HashSet<Integer>, EventError>;

    fn handle(&mut self, msg: LookupChats, _: &mut Self::Context) -> Self::Result {
        Ok(self.lookup_chats(msg.0))
    }
}

impl Handler<LookupChannels> for UsersActor {
    type Result = Result<HashSet<Integer>, EventError>;

    fn handle(&mut self, msg: LookupChannels, _: &mut Self::Context) -> Self::Result {
        Ok(self.lookup_channels(msg.0))
    }
}
