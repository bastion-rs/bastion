#[derive(PartialEq, PartialOrd)]
/// An enum that specifies a lifecycle of the message that has
/// been sent by the actor in the system.
///
/// The whole lifecycle of the message can be described by the
/// next schema:
///
///    +------ Message processed ----+
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
pub(crate) enum ActorState {
    /// The first state for actors. This state is the initial point
    /// after being created or added to the Bastion node in runtime.
    /// At this stage, actor isn't doing any useful job and retrieving
    /// any messages from other parts of the cluster yet.
    /// However, it can do some initialization steps (e.g. register itself
    /// in dispatchers or adding the initial data to the local state),
    /// before being available to the rest of the cluster.
    Init,
    /// Remote or local state synchronization. this behaves like a half state
    /// to converging consensus between multiple actor states.
    Sync,
    /// The main state in which actor can stay for indefinite amount of time.
    /// During this state, actor doing useful work (e.g. processing the incoming
    /// message from other actors) that doesn't require any asynchronous calls.
    Scheduled,
    /// Special kind of the scheduled state which help to understand that
    /// actor is awaiting for other futures or response messages from other
    /// actors in the Bastion cluster.
    Awaiting,
    /// Actor has been stopped by the system or a user's call.
    Stopped,
    /// Actor has been terminated by the system or a user's call.
    Terminated,
    /// Actor stopped doing any useful work because of a raised panic
    /// or user's error during the execution.
    Failed,
    /// Actor has completed an execution with the success.
    Finished,
    /// The deinitialization state for the actor. During this stage the actor
    /// must unregister itself from the node, used dispatchers and any other
    /// parts where was initialized in the beginning. Can contain an additional
    /// user logic before being removed from the cluster.
    Deinit,
    /// The final state of the actor. The actor can be removed
    /// gracefully from the node, because is not available anymore.
    Removed,
}
