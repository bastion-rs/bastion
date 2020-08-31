#[derive(PartialEq, PartialOrd)]
/// An enum that specifies a lifecycle of the message that has
/// been sent by the actor in the system.
///
/// The whole lifecycle of the message can be described by the
/// next schema:
///
///    +------ Message processed -----+
///    ↓                             |
/// Scheduled -> Sent -> Awaiting ---+
///               ↑           |   
///               +-- Retry --+
///
pub(crate) enum MailboxState {
    /// Mailbox has been scheduled
    Scheduled,
    /// Message has been sent to destination
    Sent,
    /// Ack has currently been awaited
    Awaiting,
}

#[derive(PartialEq, PartialOrd)]
/// Special kind of enum that describes possible states of
/// the actor in the system.  
///
/// The whole state machine of the actor can be represented by
/// the following schema:
///
/// Init -> Sync -> Scheduled -> Awaiting -> Deinit
///                    ↑            |   
///                    +------------+
///
pub(crate) enum ActorState {
    /// This is the first state for the actors,
    /// right after the creation, but the actors wasn't started retrieving
    /// messages and doing work (e.g. registering themselves in dispatchers)
    Init,
    /// Remote or local state synchronization, this behaves like a half state
    /// to converging consensus between multiple actor states.
    Sync,
    /// Currently scheduler is processing this actor's mailbox.
    Scheduled,
    /// Answer is awaited currently.
    Awaiting,
    /// State representing removing the actors from the cluster, unregistering
    /// from dispatchers, and started to hit their etc.
    Deinit,
}
