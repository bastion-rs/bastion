use super::mailbox::*;
use crate::message::*;
use crate::routing::path::*;
use std::sync::Arc;
use lever::sync::atomics::AtomicBox;
use crate::actor::state_codes::ActorState;

pub struct ActorCell {}

///
/// State data of the actor
pub struct ActorStateData<T>
where
    T: TypedMessage,
{
    /// Mailbox of the actor
    mailbox: MailboxTx<T>,

    /// State of the actor
    state: Arc<AtomicBox<ActorState>>
}

impl<T> ActorStateData<T>
where
    T: TypedMessage,
{
    pub(crate) fn new(_path: ActorPath) -> Self {
        todo!()
    }
}
