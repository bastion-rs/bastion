use crate::system::global_state::GlobalState;

#[derive(Debug)]
/// An implementation of the Bastion's node. By default
/// it's available as a singleton object. The node stores all
/// required information for running various actor implementations,
///
/// Out-of-the-box it also provides:
/// - Adding actor definitions in runtime
/// - API for starting, stopping or terminating actors
/// - Global state available to all actors
/// - Message dispatching
///
pub struct Node {
    global_state: GlobalState,
}

impl Node {
    /// Returns a new instance of the Bastion Node.
    pub(crate) fn new() -> Self {
        let global_state = GlobalState::new();

        Node { global_state }
    }

    // TODO: Add errors handling?
    /// Initializes the Bastion node if it hasn't already been done.
    pub async fn init(&self) {}
}
