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

    //////////////////////
    ///// Actor state machine
    //////////////////////

    pub(crate) fn set_init(&self) {
        self.state.replace_with(|_| ActorState::Init);
    }

    pub(crate) fn is_init(&self) -> bool {
        *self.state.get() == ActorState::Init
    }

    pub(crate) fn set_sync(&self) {
        self.state.replace_with(|_| ActorState::Sync);
    }

    pub(crate) fn is_sync(&self) -> bool {
        *self.state.get() == ActorState::Sync
    }

    pub(crate) fn set_scheduled(&self) {
        self.state.replace_with(|_| ActorState::Scheduled);
    }

    pub(crate) fn is_scheduled(&self) -> bool {
        *self.state.get() == ActorState::Scheduled
    }

    pub(crate) fn set_awaiting(&self) {
        self.state.replace_with(|_| ActorState::Awaiting);
    }

    pub(crate) fn is_awaiting(&self) -> bool {
        *self.state.get() == ActorState::Awaiting
    }

    pub(crate) fn set_deinit(&self) {
        self.state.replace_with(|_| ActorState::Deinit);
    }

    pub(crate) fn is_deinit(&self) -> bool {
        *self.state.get() == ActorState::Deinit
    }
}
