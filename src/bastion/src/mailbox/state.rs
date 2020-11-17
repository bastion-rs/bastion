use crossbeam::atomic::AtomicCell;

// Mailbox state holder
#[derive(Debug)]
pub(crate) struct MailboxState {
    inner: AtomicCell<InnerState>,
}

#[derive(Debug, Copy, Clone, PartialEq, PartialOrd)]
/// An enum that specifies a lifecycle of the message that has
/// been sent by the actor in the system.
///
/// The whole lifecycle of the message can be described by the
/// next schema:
/// ```ignore
///
///    +---- Message processed -----+
///    ↓                            |
/// Scheduled -> Sent -> Awaiting --+
///               ↑           |
///               +-- Retry --+
///
/// ```
enum InnerState {
    /// Message has been scheduled to delivery
    Scheduled,
    /// Message has been sent to destination
    Sent,
    /// Ack has currently been awaited
    Awaiting,
}

impl MailboxState {
    pub(crate) fn new() -> Self {
        MailboxState {
            inner: AtomicCell::new(InnerState::Scheduled),
        }
    }

    pub(crate) fn set_scheduled(&self) {
        self.inner.store(InnerState::Scheduled)
    }

    pub(crate) fn set_sent(&self) {
        self.inner.store(InnerState::Sent)
    }

    pub(crate) fn set_awaiting(&self) {
        self.inner.store(InnerState::Awaiting)
    }

    pub(crate) fn is_scheduled(&self) -> bool {
        self.inner.load() == InnerState::Scheduled
    }

    pub(crate) fn is_sent(&self) -> bool {
        self.inner.load() == InnerState::Sent
    }

    pub(crate) fn is_awaiting(&self) -> bool {
        self.inner.load() == InnerState::Awaiting
    }
}

#[cfg(test)]
mod tests {
    use crate::mailbox::state::MailboxState;

    #[test]
    fn test_normal_path() {
        let state = MailboxState::new();

        assert_eq!(state.is_scheduled(), true);

        state.set_sent();
        assert_eq!(state.is_sent(), true);

        state.set_awaiting();
        assert_eq!(state.is_awaiting(), true);

        state.set_scheduled();
        assert_eq!(state.is_scheduled(), true);
    }

    #[test]
    fn test_path_with_retry() {
        let state = MailboxState::new();

        assert_eq!(state.is_scheduled(), true);

        state.set_sent();
        assert_eq!(state.is_sent(), true);

        state.set_awaiting();
        assert_eq!(state.is_awaiting(), true);

        state.set_sent();
        assert_eq!(state.is_sent(), true);

        state.set_awaiting();
        assert_eq!(state.is_awaiting(), true);

        state.set_scheduled();
        assert_eq!(state.is_scheduled(), true);
    }
}
