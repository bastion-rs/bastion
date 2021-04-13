//!
//! Describes the error types that may happen within bastion.
//! Given Bastion has a let it crash strategy, most error aren't noticeable.
//! A ReceiveError may however be raised when calling try_recv() or try_recv_timeout()
//! More errors may happen in the future.

use crate::envelope::Envelope;
use crate::message::Msg;
use crate::system::STRING_INTERNER;
use crate::{distributor::Distributor, message::BastionMessage};
use futures::channel::mpsc::TrySendError;
use std::fmt::Debug;
use std::time::Duration;
use thiserror::Error;

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

#[derive(Error, Debug)]
/// `SendError`s occur when a message couldn't be dispatched through a distributor
pub enum SendError {
    #[error("couldn't send message. Channel Disconnected.")]
    /// Channel has been closed before we could send a message
    Disconnected(Msg),
    #[error("couldn't send message. Channel is Full.")]
    /// Channel is full, can't send a message
    Full(Msg),
    #[error("couldn't send a message I should have not sent. {0}")]
    /// This error is returned when we try to send a message
    /// that is not a BastionMessage::Message variant
    Other(anyhow::Error),
    #[error("No available Distributor matching {0}")]
    /// The distributor we're trying to dispatch messages to is not registered in the system
    NoDistributor(String),
    #[error("Distributor has 0 Recipients")]
    /// The distributor we're trying to dispatch messages to has no recipients
    EmptyRecipient,
}

impl From<TrySendError<Envelope>> for SendError {
    fn from(tse: TrySendError<Envelope>) -> Self {
        let is_disconnected = tse.is_disconnected();
        match tse.into_inner().msg {
            BastionMessage::Message(msg) => {
                if is_disconnected {
                    Self::Disconnected(msg)
                } else {
                    Self::Full(msg)
                }
            }
            other => Self::Other(anyhow::anyhow!("{:?}", other)),
        }
    }
}

impl From<Distributor> for SendError {
    fn from(distributor: Distributor) -> Self {
        Self::NoDistributor(STRING_INTERNER.resolve(distributor.interned()).to_string())
    }
}
