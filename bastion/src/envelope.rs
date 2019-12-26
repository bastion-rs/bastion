//!
//! A envelope wraps messages and signs them to help user identify message senders
//! and instruct Bastion how to send messages back to them

use crate::broadcast::Sender;
use crate::message::{BastionMessage, Message, Msg};
use crate::path::BastionPath;
use crate::ref_addr::RefAddr;
use std::sync::Arc;

#[derive(Debug)]
pub(crate) struct Envelope {
    pub(crate) msg: BastionMessage,
    pub(crate) sign: RefAddr,
}

#[derive(Debug)]
/// A struct containing a message and its sender signature
///
/// # Example
///
/// ```rust
/// # use bastion::prelude::*;
/// #
/// # fn main() {
///     # Bastion::init();
///     #
/// Bastion::children(|children| {
///     children.with_exec(|ctx: BastionContext| {
///         async move {
///             let msg: SignedMessage = ctx.recv().await?;
///
///             Ok(())
///         }
///     })
/// }).expect("Couldn't create the children group.");
///     #
///     # Bastion::start();
///     # Bastion::stop();
///     # Bastion::block_until_stopped();
/// # }
/// ```
pub struct SignedMessage {
    pub(crate) msg: Msg,
    pub(crate) sign: RefAddr,
}

impl SignedMessage {
    pub(crate) fn new(msg: Msg, sign: RefAddr) -> Self {
        SignedMessage { msg, sign }
    }

    #[doc(hidden)]
    pub fn extract(self) -> (Msg, RefAddr) {
        (self.msg, self.sign)
    }

    /// Returns a message signature to identify the message sender
    ///
    /// # Example
    ///
    /// ```rust
    /// # use bastion::prelude::*;
    /// #
    /// # fn main() {
    ///     # Bastion::init();
    ///     #
    /// Bastion::children(|children| {
    ///     children.with_exec(|ctx: BastionContext| {
    ///         async move {
    ///             let msg: SignedMessage = ctx.recv().await?;
    ///             println!("received message from {:?}", msg.signature().path());
    ///             Ok(())
    ///         }
    ///     })
    /// }).expect("Couldn't create the children group.");
    ///     #
    ///     # Bastion::start();
    ///     # Bastion::stop();
    ///     # Bastion::block_until_stopped();
    /// # }
    /// ```
    pub fn signature(&self) -> &RefAddr {
        &self.sign
    }
}

impl Envelope {
    pub(crate) fn new(msg: BastionMessage, path: Arc<BastionPath>, sender: Sender) -> Self {
        Envelope {
            msg,
            sign: RefAddr::new(path, sender),
        }
    }

    pub(crate) fn new_with_sign(msg: BastionMessage, sign: RefAddr) -> Self {
        Envelope { msg, sign }
    }

    pub(crate) fn from_dead_letters(msg: BastionMessage) -> Self {
        Envelope {
            msg,
            sign: RefAddr::dead_letters(),
        }
    }

    pub(crate) fn try_clone(&self) -> Option<Self> {
        self.msg.try_clone().map(|msg| Envelope {
            msg,
            sign: self.sign.clone(),
        })
    }

    pub(crate) fn into_msg<M: Message>(self) -> Option<M> {
        self.msg.into_msg()
    }
}
