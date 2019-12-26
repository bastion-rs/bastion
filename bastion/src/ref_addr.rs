//!
//! A ref addr allows users to send messages to element refs

use crate::broadcast::Sender;
use crate::path::BastionPath;
use crate::system::SYSTEM;
use std::sync::Arc;

#[derive(Debug, Clone)]
/// Message signature used to identify message sender and send messages to it.
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
///             // Wait for a message to be received...
///             let msg: SignedMessage = ctx.recv().await?;
///             
///             ctx.tell(msg.signature(), "reply").expect("Unable to reply");
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
pub struct RefAddr {
    path: Arc<BastionPath>,
    // FIXME: remove pub(crate), only needed for a temporary Broadcast::extract_channel
    pub(crate) sender: Sender,
}

impl RefAddr {
    pub(crate) fn new(path: Arc<BastionPath>, sender: Sender) -> Self {
        RefAddr { path, sender }
    }

    pub(crate) fn dead_letters() -> Self {
        Self::new(
            SYSTEM.dead_letters().path().clone(),
            SYSTEM.dead_letters().sender().clone(),
        )
    }

    /// Checks whether the sender is identified.
    /// Usually anonymous sender means messages sent by
    /// [broadcast][crate::Bastion::broadcast()] and it's other methods implied to
    /// be called outside of the Bastion context.
    ///
    /// # Example
    ///
    /// ```rust
    /// # use bastion::prelude::*;
    /// #
    /// # fn main() {
    ///     # Bastion::init();
    ///     #
    ///     # let children_ref = Bastion::children(|children| children).unwrap();
    /// let msg = "A message containing data.";
    /// children_ref.broadcast(msg).expect("Couldn't send the message.");
    ///
    ///     # Bastion::children(|children| {
    ///         # children.with_exec(|ctx: BastionContext| {
    ///             # async move {
    /// msg! { ctx.recv().await?,
    ///     ref msg: &'static str => {
    ///         assert!(signature!().is_sender_identified());
    ///     };
    ///     // We are only sending a `&'static str` in this
    ///     // example, so we know that this won't happen...
    ///     _: _ => ();
    /// }
    ///                 #
    ///                 # Ok(())
    ///             # }
    ///         # })
    ///     # }).unwrap();
    ///     #
    ///     # Bastion::start();
    ///     # Bastion::stop();
    ///     # Bastion::block_until_stopped();
    /// # }
    /// ```
    pub fn is_sender_identified(&self) -> bool {
        self.path.is_dead_letters()
    }

    /// Returns `BastionPath` of a sender
    ///
    /// # Example
    ///
    /// ```rust
    /// # use bastion::prelude::*;
    /// #
    /// # fn main() {
    ///     # Bastion::init();
    ///     #
    ///     # let children_ref = Bastion::children(|children| children).unwrap();
    /// let msg = "A message containing data.";
    /// children_ref.broadcast(msg).expect("Couldn't send the message.");
    ///
    ///     # Bastion::children(|children| {
    ///         # children.with_exec(|ctx: BastionContext| {
    ///             # async move {
    /// msg! { ctx.recv().await?,
    ///     ref msg: &'static str => {
    ///         let path = signature!().path();
    ///         assert!(path.is_dead_letters());
    ///     };
    ///     // We are only sending a `&'static str` in this
    ///     // example, so we know that this won't happen...
    ///     _: _ => ();
    /// }
    ///                 #
    ///                 # Ok(())
    ///             # }
    ///         # })
    ///     # }).unwrap();
    ///     #
    ///     # Bastion::start();
    ///     # Bastion::stop();
    ///     # Bastion::block_until_stopped();
    /// # }
    /// ```
    pub fn path(&self) -> &Arc<BastionPath> {
        &self.path
    }

    pub(crate) fn sender(&self) -> &Sender {
        &self.sender
    }
}
