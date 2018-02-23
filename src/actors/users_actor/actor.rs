use actix::{Actor, AsyncContext, Context, Handler};
use failure::Fail;
use futures::{Future, Stream};
use futures::stream::iter_ok;

use error::{EventError, EventErrorKind};
use actors::db_actor::messages::GetUsersWithChats;
use models::user::User;
use models::chat::Chat;
use super::{UserState, UsersActor};
use super::messages::*;

impl Actor for UsersActor {
    type Context = Context<Self>;

    fn started(&mut self, ctx: &mut Self::Context) {
        ctx.add_stream(
            self.db
                .call_fut(GetUsersWithChats)
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
        )
    }
}

impl Handler<Result<TouchUser, EventError>> for UsersActor {
    type Result = Result<UserState, ()>;

    fn handle(
        &mut self,
        msg: Result<TouchUser, EventError>,
        _: &mut Self::Context,
    ) -> Self::Result {
        msg.map(|msg| self.touch_user(msg.0, msg.1)).map_err(|_| ())
    }
}

impl Handler<TouchUser> for UsersActor {
    type Result = Result<UserState, EventError>;

    fn handle(&mut self, msg: TouchUser, _: &mut Self::Context) -> Self::Result {
        Ok(self.touch_user(msg.0, msg.1))
    }
}
