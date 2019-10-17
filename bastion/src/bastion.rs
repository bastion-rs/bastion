use crate::registry::Registry;
use crate::supervisor::Supervisor;
use crate::system::System;
use futures::channel::mpsc::UnboundedSender;
use lazy_static::lazy_static;

lazy_static! {
    pub(super) static ref SYSTEM: UnboundedSender<Supervisor> = System::start();
    pub(super) static ref REGISTRY: Registry = Registry::new();
}

pub struct Bastion {
    // TODO: ...
}

impl Bastion {
    pub fn supervisor() -> Supervisor {
        Supervisor::new()
    }
}
