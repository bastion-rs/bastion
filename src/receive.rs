//!
//! Implementations for message gating, receiving and destination definitions.

use crate::child::Message;

///
/// In-process message receive implementation
pub struct Receive<T>(pub Option<T>);

impl<T: 'static + Clone> From<Box<dyn Message>> for Receive<T> {
    fn from(message: Box<dyn Message>) -> Self {
        Receive(message.as_any().downcast_ref::<T>().cloned())
    }
}
