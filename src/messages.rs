//!
//! Generic message types for management of runtime, processes and layers.
//!
//! To make all the messages effective processes should bind to system with
//! [hook](../context/struct.BastionContext.html#method.hook)
//! or [blocking_hook](../context/struct.BastionContext.html#method.blocking_hook) based on how they are going to be managed.
use std::any::Any;

///
/// [PoisonPill] is used to kill a process
///
/// If a process receives [PoisonPill], it is going to shutdown.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct PoisonPill;

impl PoisonPill {
    pub fn new() -> Box<PoisonPill> {
        Box::new(PoisonPill::default())
    }

    pub fn as_any(&self) -> &dyn Any {
        self
    }
}
