use std::{fmt::Debug, sync::Arc};

use futures::channel::mpsc;
use lasso::{Spur, ThreadedRodeo};
use once_cell::sync::Lazy;

use crate::{
    child_ref::SendError,
    message::{AnswerSender, Message},
    prelude::{RefAddr, SignedMessage},
    system::SYSTEM,
};

pub(crate) static INTERNER: Lazy<Arc<ThreadedRodeo>> = Lazy::new(|| Arc::new(Default::default()));

// Copy is fine here because we're working
// with interned strings here
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Distributor(Spur);

impl Distributor {
    pub fn named(name: impl AsRef<str>) -> Self {
        Self(INTERNER.get_or_intern(name.as_ref()))
    }

    pub fn interned(&self) -> &Spur {
        &self.0
    }

    pub fn ask_one(&self, question: impl Message) {
        // wrap it into a question payload
        let payload = Payload::Question {
            message: Box::new(question),
            reply_to: None,
        };
        // wrap it into an envelope
        let envelope = Envelope::Letter(payload);
        // send it
        let _ = self.send(envelope);
    }

    fn send(self, envelope: Envelope) -> Result<(), SendError> {
        let global_dispatcher = SYSTEM.dispatcher();
        global_dispatcher.distribute(self, envelope)
    }
}

pub enum Envelope {
    Letter(Payload),
    Leaflet(ClonePayload),
}

#[derive(Debug)]
pub struct MultiSender(mpsc::Sender<SignedMessage>, RefAddr);
pub enum ClonePayload {
    Statement(Arc<dyn Message>),
    Question {
        message: Arc<dyn Message>,
        reply_to: Option<MultiSender>,
    },
}

pub enum Payload {
    Statement(Box<dyn Message>),
    Question {
        message: Box<dyn Message>,
        reply_to: Option<AnswerSender>,
    },
}
