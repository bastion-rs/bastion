/// Module that holds state machine implementation for the Bastion actor.
/// Not available for changes for crate users, but used internally for clean
/// actor's state transitions in the readable and the debuggable manner.
///
/// The whole state machine of the actor can be represented by the
/// following schema:
///
///                            +---> Stopped ----+
///                            |                 |
///                            |                 |
/// Init -> Sync -> Scheduled -+---> Terminated -+---> Deinit -> Removed
///                  ↑     |   |                 |
///                  |     ↓   |                 |
///                 Awaiting   +---> Failed -----+
///                            |                 |
///                            |                 |
///                            +---> Finished ---+
///
///
/// The transitions between the states is called by the actor's context
/// internally and aren't available to use and override by crate users.
///
use std::sync::Arc;

use crossbeam::atomic::AtomicCell;

use crate::actor::state::InnerState::Init;

// Actor state holder
#[derive(Debug)]
pub(crate) struct ActorState {
    inner: AtomicCell<InnerState>,
}

#[derive(Debug, Copy, Clone, PartialEq, PartialOrd)]
enum InnerState {
    /// The first state for actors. This state is the initial point after
    /// being created or added to the Bastion node in runtime. At this stage,
    /// actor isn't doing any useful job and retrieving any messages from other
    /// parts of the cluster yet.
    /// However, it can do some initialization steps (e.g. register itself
    /// in dispatchers or adding the initial data to the local state),
    /// before being available to the rest of the cluster.
    Init,
    /// An intermediate state that used for sychronization purposes. Useful
    /// for cases when necessary to have a consensus between multiple actors.
    Sync,
    /// The main state in which actor can stay for indefinite amount of time.
    /// During this state, actor does useful work (e.g. processing the incoming
    /// messages from other actors) that doesn't require any asynchronous calls.
    Scheduled,
    /// Special kind of the actor's state that helps to understand that actor
    /// is awaiting for other futures (e.g. I/O, network) or awaiting a response
    /// from other actor in the Bastion cluster.
    Awaiting,
    /// Actor has been stopped by the system or a user's call.
    Stopped,
    /// Actor has been terminated by the system or a user's call.
    Terminated,
    /// Actor has been stopped because of a raised panic during the execution
    /// or returned an error.
    Failed,
    /// Actor has completed code execution with the success.
    Finished,
    /// The deinitialization state for the actor. During this stage the actor
    /// must unregister itself from the node, used dispatchers and any other
    /// parts where was initialized in the beginning. Can contain an additional
    /// user logic before being removed from the cluster.
    Deinit,
    /// The final state of the actor. The actor can be removed gracefully from
    /// the Bastion node, without any negative impact to the cluster.
    Removed,
}

impl ActorState {
    pub(crate) fn new() -> Self {
        ActorState {
            inner: AtomicCell::new(InnerState::Init),
        }
    }

    pub(crate) fn set_sync(&self) {
        self.inner.store(InnerState::Sync);
    }

    pub(crate) fn set_scheduled(&self) {
        self.inner.store(InnerState::Scheduled);
    }

    pub(crate) fn set_awaiting(&self) {
        self.inner.store(InnerState::Awaiting);
    }

    pub(crate) fn set_stopped(&self) {
        self.inner.store(InnerState::Stopped);
    }

    pub(crate) fn set_terminated(&self) {
        self.inner.store(InnerState::Terminated);
    }

    pub(crate) fn set_failed(&self) {
        self.inner.store(InnerState::Failed);
    }

    pub(crate) fn set_finished(&self) {
        self.inner.store(InnerState::Finished);
    }

    pub(crate) fn set_deinit(&self) {
        self.inner.store(InnerState::Deinit);
    }

    pub(crate) fn set_removed(&self) {
        self.inner.store(InnerState::Removed);
    }

    pub(crate) fn is_init(&self) -> bool {
        self.inner.load() == InnerState::Init
    }

    pub(crate) fn is_sync(&self) -> bool {
        self.inner.load() == InnerState::Sync
    }

    pub(crate) fn is_scheduled(&self) -> bool {
        self.inner.load() == InnerState::Scheduled
    }

    pub(crate) fn is_awaiting(&self) -> bool {
        self.inner.load() == InnerState::Awaiting
    }

    pub(crate) fn is_stopped(&self) -> bool {
        self.inner.load() == InnerState::Stopped
    }

    pub(crate) fn is_terminated(&self) -> bool {
        self.inner.load() == InnerState::Terminated
    }

    pub(crate) fn is_failed(&self) -> bool {
        self.inner.load() == InnerState::Failed
    }

    pub(crate) fn is_finished(&self) -> bool {
        self.inner.load() == InnerState::Finished
    }

    pub(crate) fn is_deinit(&self) -> bool {
        self.inner.load() == InnerState::Deinit
    }

    pub(crate) fn is_removed(&self) -> bool {
        self.inner.load() == InnerState::Removed
    }
}

#[cfg(test)]
mod tests {
    use crate::actor::state::ActorState;

    #[test]
    fn test_happy_path() {
        let state = ActorState::new();

        assert_eq!(state.is_init(), true);

        state.set_sync();
        assert_eq!(state.is_sync(), true);

        state.set_scheduled();
        assert_eq!(state.is_scheduled(), true);

        state.set_finished();
        assert_eq!(state.is_finished(), true);

        state.set_deinit();
        assert_eq!(state.is_deinit(), true);

        state.set_removed();
        assert_eq!(state.is_removed(), true);
    }

    #[test]
    fn test_happy_path_with_awaiting_state() {
        let state = ActorState::new();

        assert_eq!(state.is_init(), true);

        state.set_sync();
        assert_eq!(state.is_sync(), true);

        state.set_scheduled();
        assert_eq!(state.is_scheduled(), true);

        state.set_awaiting();
        assert_eq!(state.is_awaiting(), true);

        state.set_scheduled();
        assert_eq!(state.is_scheduled(), true);

        state.set_finished();
        assert_eq!(state.is_finished(), true);

        state.set_deinit();
        assert_eq!(state.is_deinit(), true);

        state.set_removed();
        assert_eq!(state.is_removed(), true);
    }

    #[test]
    fn test_path_with_stopped_state() {
        let state = ActorState::new();

        assert_eq!(state.is_init(), true);

        state.set_sync();
        assert_eq!(state.is_sync(), true);

        state.set_scheduled();
        assert_eq!(state.is_scheduled(), true);

        state.set_stopped();
        assert_eq!(state.is_stopped(), true);

        state.set_deinit();
        assert_eq!(state.is_deinit(), true);

        state.set_removed();
        assert_eq!(state.is_removed(), true);
    }

    #[test]
    fn test_path_with_terminated_state() {
        let state = ActorState::new();

        assert_eq!(state.is_init(), true);

        state.set_sync();
        assert_eq!(state.is_sync(), true);

        state.set_scheduled();
        assert_eq!(state.is_scheduled(), true);

        state.set_terminated();
        assert_eq!(state.is_terminated(), true);

        state.set_deinit();
        assert_eq!(state.is_deinit(), true);

        state.set_removed();
        assert_eq!(state.is_removed(), true);
    }

    #[test]
    fn test_path_with_failed_state() {
        let state = ActorState::new();

        assert_eq!(state.is_init(), true);

        state.set_sync();
        assert_eq!(state.is_sync(), true);

        state.set_scheduled();
        assert_eq!(state.is_scheduled(), true);

        state.set_failed();
        assert_eq!(state.is_failed(), true);

        state.set_deinit();
        assert_eq!(state.is_deinit(), true);

        state.set_removed();
        assert_eq!(state.is_removed(), true);
    }
}
