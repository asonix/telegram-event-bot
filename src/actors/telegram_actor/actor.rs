use actix::{Actor, Context, Handler};

use super::messages::*;
use super::TelegramActor;

impl Actor for TelegramActor {
    type Context = Context<Self>;
}

impl Handler<NotifyEvent> for TelegramActor {
    type Result = ();

    fn handle(&mut self, msg: NotifyEvent, _: &mut Self::Context) -> Self::Result {
        self.notify_event(msg.0);
    }
}

impl Handler<EventOver> for TelegramActor {
    type Result = ();

    fn handle(&mut self, msg: EventOver, _: &mut Self::Context) -> Self::Result {
        self.query_events(msg.event_id, msg.system_id);
    }
}

impl Handler<AnswerTitle> for TelegramActor {
    type Result = ();

    fn handle(&mut self, msg: AnswerTitle, _: &mut Self::Context) -> Self::Result {
        self.answer_title(msg.chat_id)
    }
}

impl Handler<FailedAnswerDescription> for TelegramActor {
    type Result = ();

    fn handle(&mut self, msg: FailedAnswerDescription, _: &mut Self::Context) -> Self::Result {
        self.failed_answer_description(msg.chat_id)
    }
}

impl Handler<AnswerDescription> for TelegramActor {
    type Result = ();

    fn handle(&mut self, msg: AnswerDescription, _: &mut Self::Context) -> Self::Result {
        self.answer_description(msg.chat_id)
    }
}

impl Handler<FailedAnswerDate> for TelegramActor {
    type Result = ();

    fn handle(&mut self, msg: FailedAnswerDate, _: &mut Self::Context) -> Self::Result {
        self.failed_answer_date(msg.chat_id)
    }
}

impl Handler<AnswerDate> for TelegramActor {
    type Result = ();

    fn handle(&mut self, msg: AnswerDate, _: &mut Self::Context) -> Self::Result {
        self.answer_date(msg.chat_id)
    }
}

impl Handler<FailedAnswerEnd> for TelegramActor {
    type Result = ();

    fn handle(&mut self, msg: FailedAnswerEnd, _: &mut Self::Context) -> Self::Result {
        self.failed_answer_end(msg.chat_id)
    }
}

impl Handler<AnswerEnd> for TelegramActor {
    type Result = ();

    fn handle(&mut self, msg: AnswerEnd, _: &mut Self::Context) -> Self::Result {
        self.answer_end(msg.chat_id)
    }
}

impl Handler<FailedAnswerHost> for TelegramActor {
    type Result = ();

    fn handle(&mut self, msg: FailedAnswerHost, _: &mut Self::Context) -> Self::Result {
        self.failed_answer_hosts(msg.chat_id)
    }
}

impl Handler<AnswerHost> for TelegramActor {
    type Result = ();

    fn handle(&mut self, msg: AnswerHost, _: &mut Self::Context) -> Self::Result {
        self.answer_hosts(msg.chat_id)
    }
}

impl Handler<FailedAnswerFinalize> for TelegramActor {
    type Result = ();

    fn handle(&mut self, msg: FailedAnswerFinalize, _: &mut Self::Context) -> Self::Result {
        self.failed_answer_finalize(msg.chat_id)
    }
}

impl Handler<AnswerFinalize> for TelegramActor {
    type Result = ();

    fn handle(&mut self, msg: AnswerFinalize, _: &mut Self::Context) -> Self::Result {
        self.answer_finalize(msg.chat_id)
    }
}
