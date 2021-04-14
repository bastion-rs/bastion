use std::sync::Arc;

use async_channel::unbounded;

use crate::actor::local_state::LocalState;
use crate::actor::state::ActorState;
use crate::mailbox::traits::TypedMessage;
use crate::mailbox::Mailbox;
use crate::routing::path::ActorPath;

/// A structure that defines actor's state, mailbox with  
/// messages and a local storage for user's data.
///
/// Each actor in Bastion has an attached context which
/// helps to understand what is the type of actor has been
/// launched in the system, its path, current execution state
/// and various data that can be attached to it.
pub struct Context {
    /// Path to the actor in the system
    path: Arc<ActorPath>,
    /// Mailbox of the actor
    //mailbox: Mailbox<TypedMessage>,
    /// Local storage for actor's data
    local_state: LocalState,
    /// Current execution state of the actor
    internal_state: ActorState,
}

impl Context {
    // FIXME: Pass the correct system_rx instead of the fake one
    pub(crate) fn new(path: ActorPath) -> Self {
        //let (_system_tx, system_rx) = unbounded();
        // let mailbox = Mailbox::new(system_rx);

        let path = Arc::new(path);
        let local_state = LocalState::new();
        let internal_state = ActorState::new();

        Context {
            path,
            //mailbox,
            local_state,
            internal_state,
        }
    }
}
