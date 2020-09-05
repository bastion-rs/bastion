use super::mailbox::*;
use crate::actor::state_codes::ActorState;
use crate::message::*;
use crate::routing::path::*;
use lever::sync::atomics::AtomicBox;
use std::sync::Arc;

/// A structure that defines actor's state, mailbox with  
/// messages and a local storage for user's data.
///
/// Each actor in Bastion has an attached context which
/// helps to understand what is the type of actor has been
/// launched in the system, its path, current execution state
/// and various data that can be attached to it.
pub struct Context<T>
where
    T: TypedMessage,
{
    /// Path to the actor in the system
    path: ActorPath,
    /// Mailbox of the actor
    mailbox: MailboxTx<T>,
    /// Current execution state of the actor
    state: Arc<AtomicBox<ActorState>>,
}

impl<T> Context<T>
where
    T: TypedMessage,
{
    pub(crate) fn new(path: ActorPath) -> Self {
        let mailbox = MailboxTx: new();
        let state = Arc::new(AtomicBox::new(ActorState::Init));

        Context {
            path,
            mailbox,
            state,
        }
    }

    // Actor state machine.
    //
    // For more information about the actor's state machine
    // see the actor/state_codes.rs module.
    //

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

    pub(crate) fn set_stopped(&self) {
        self.state.replace_with(|_| ActorState::Stopped);
    }

    pub(crate) fn is_stopped(&self) -> bool {
        *self.state.get() == ActorState::Stopped
    }

    pub(crate) fn set_terminated(&self) {
        self.state.replace_with(|_| ActorState::Terminated);
    }

    pub(crate) fn is_terminated(&self) -> bool {
        *self.state.get() == ActorState::Terminated
    }

    pub(crate) fn set_failed(&self) {
        self.state.replace_with(|_| ActorState::Failed);
    }

    pub(crate) fn is_failed(&self) -> bool {
        *self.state.get() == ActorState::Failed
    }

    pub(crate) fn set_deinit(&self) {
        self.state.replace_with(|_| ActorState::Deinit);
    }

    pub(crate) fn is_deinit(&self) -> bool {
        *self.state.get() == ActorState::Deinit
    }

    pub(crate) fn set_finished(&self) {
        self.state.replace_with(|_| ActorState::Finished);
    }

    pub(crate) fn is_finished(&self) -> bool {
        *self.state.get() == ActorState::Finished
    }
}
