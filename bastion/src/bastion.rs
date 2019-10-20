use crate::broadcast::{BastionMessage, Broadcast, Parent};
use crate::children::Closure;
use crate::context::NIL_ID;
use crate::registry::Registry;
use crate::supervisor::Supervisor;
use crate::system::{STARTED, SYSTEM};
use lazy_static::lazy_static;
use std::thread;

lazy_static! {
    pub(super) static ref REGISTRY: Registry = Registry::new();
}

pub struct Bastion {
    // TODO: ...
}

impl Bastion {
    pub fn init() {
        std::panic::set_hook(Box::new(|_| ()));

        // NOTE: this is just to make sure that SYSTEM has been initialized by lazy_static
        SYSTEM.is_closed();
    }

    pub fn supervisor<S>(init: S)
    where
        S: FnOnce(Supervisor) -> Supervisor,
    {
        let parent = Parent::system();
        let bcast = Broadcast::new(parent);

        let supervisor = Supervisor::new(bcast);
        let supervisor = init(supervisor);
        let msg = BastionMessage::deploy_supervisor(supervisor);
        // FIXME: Err(Error)
        SYSTEM.unbounded_send(msg).ok();
    }

    pub fn children<F>(thunk: F, redundancy: usize)
    where
        F: Closure,
    {
        // FIXME: panics
        REGISTRY
            .get_supervisor(&NIL_ID)
            .unwrap()
            .children(thunk, redundancy);
    }

    pub fn start() {
        let msg = BastionMessage::start();
        // FIXME: Err(Error)
        SYSTEM.unbounded_send(msg).ok();

        loop {
            // FIXME: panics
            let started = STARTED.clone().lock().wait().unwrap();
            if *started {
                return;
            }

            thread::yield_now();
        }
    }
}
