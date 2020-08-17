use super::mailbox::*;
use crate::message::*;
use crate::routing::path::*;

pub struct ActorCell {}

///
/// State of the actor
pub struct ActorState<T>
where
    T: TypedMessage,
{
    mailbox: MailboxTx<T>,
}

impl<T> ActorState<T>
where
    T: TypedMessage,
{
    pub(crate) fn new(_path: ActorPath) -> Self {
        todo!()
    }
}
