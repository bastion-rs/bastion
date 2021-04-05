use std::{fmt::Debug, sync::Arc};

use futures::channel::mpsc;
use lasso::Spur;

use crate::{
    child_ref::SendError,
    message::{AnswerSender, Message},
    prelude::{RefAddr, SignedMessage},
    system::{STRING_INTERNER, SYSTEM},
};

// Copy is fine here because we're working
// with interned strings here
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Distributor(Spur);

impl Distributor {
    pub fn named(name: impl AsRef<str>) -> Self {
        Self(STRING_INTERNER.get_or_intern(name.as_ref()))
    }

    pub fn interned(&self) -> &Spur {
        &self.0
    }

    // todo: this will probably return a future<Output=impl Message> or something
    pub fn ask_one(&self, question: impl Message) {
        // wrap it into a question payload
        let payload = Payload::Question {
            message: Box::new(question),
            reply_to: None,
        };
        // wrap it into an envelope
        let envelope = Envelope::Letter(payload);
        // send it
        self.send(envelope).unwrap();
    }

    pub fn tell_one(&self, message: impl Message) {
        let payload = Payload::Statement(Box::new(message));
        let envelope = Envelope::Letter(payload);
        self.send(envelope).unwrap()
    }

    pub fn tell_everyone(&self, message: impl Message) {
        let payload = ClonePayload::Statement(Arc::new(message));
        let envelope = Envelope::Leaflet(payload);
        self.send(envelope).unwrap()
    }

    fn send<M: Message>(self, envelope: Envelope<M>) -> Result<(), SendError> {
        let global_dispatcher = SYSTEM.dispatcher();
        global_dispatcher.distribute(self, envelope)
    }
}

pub enum Envelope<M: Message> {
    Letter(Payload<M>),
    Leaflet(ClonePayload<M>),
}

#[derive(Debug)]
pub struct MultiSender(mpsc::Sender<SignedMessage>, RefAddr);
pub enum ClonePayload<M: Message> {
    Statement(Arc<M>),
    Question {
        message: Arc<M>,
        reply_to: Option<MultiSender>,
    },
}

pub enum Payload<M: Message> {
    Statement(M),
    Question {
        message: M,
        reply_to: Option<AnswerSender>,
    },
}
