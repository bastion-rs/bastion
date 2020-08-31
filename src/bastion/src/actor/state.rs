use super::mailbox::*;
use crate::message::*;
use crate::routing::path::*;
use std::sync::Arc;
use std::sync::atomic::{AtomicU16, Ordering};

pub struct ActorCell {}

///
/// State of the actor
pub struct ActorState<T>
where
    T: TypedMessage,
{
    /// Mailbox of the actor
    mailbox: MailboxTx<T>,

    /// State of the actor
    state: Arc<AtomicU16>
}

impl<T> ActorState<T>
where
    T: TypedMessage,
{
    pub(crate) fn new(_path: ActorPath) -> Self {
        todo!()
    }
}
