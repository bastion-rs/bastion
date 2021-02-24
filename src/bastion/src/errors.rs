//!
//! Describes the error types that may happen within bastion.
//! Given Bastion has a let it crash strategy, most error aren't noticeable.
//! A ReceiveError may however be raised when calling try_recv() or try_recv_timeout()
//! More errors may happen in the future.

use std::time::Duration;

#[derive(Debug)]
/// These errors happen
/// when [`try_recv`] or [`try_recv_timeout`] are invoked
///
/// [`try_recv`]: crate::context::BastionContext::try_recv
/// [`try_recv_timeout`]: crate::context::BastionContext::try_recv_timeout
pub enum ReceiveError {
    /// We didn't receive a message on time
    Timeout(Duration),
    /// Generic error. Not used yet
    Other,
}
