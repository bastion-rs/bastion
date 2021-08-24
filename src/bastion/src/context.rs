//!
//! A context allows a child's future to access its received
//! messages, parent and supervisor.

use crate::child_ref::ChildRef;
use crate::children_ref::ChildrenRef;
use crate::dispatcher::{BroadcastTarget, DispatcherType, NotificationType};
use crate::envelope::{Envelope, RefAddr, SignedMessage};
use crate::global_state::GlobalState;
use crate::message::{Answer, BastionMessage, Message, Msg};
use crate::supervisor::SupervisorRef;
use crate::{prelude::ReceiveError, system::SYSTEM};

use crossbeam_queue::SegQueue;
use futures::pending;
use futures::FutureExt;
use futures_timer::Delay;
#[cfg(feature = "scaling")]
use lever::table::lotable::LOTable;
use std::fmt::{self, Display, Formatter};
use std::pin::Pin;
#[cfg(feature = "scaling")]
use std::sync::atomic::AtomicU64;
use std::{sync::Arc, time::Duration};
use tracing::{debug, trace};
use uuid::Uuid;

/// Identifier for a root supervisor and dead-letters children.
pub const NIL_ID: BastionId = BastionId(Uuid::nil());

#[derive(Hash, Eq, PartialEq, Debug, Clone)]
/// An identifier used by supervisors, children groups and
/// their elements to identify themselves, using a v4 UUID.
///
/// A `BastionId` is unique to its attached element and is
/// reset when it is restarted. A special `BastionId` exists
/// for the "system supervisor" (the supervisor created by
/// the system at startup) which is a nil UUID
/// (00000000-0000-0000-0000-000000000000).
///
/// # Example
///
/// ```rust
/// # use bastion::prelude::*;
/// #
/// # #[cfg(feature = "tokio-runtime")]
/// # #[tokio::main]
/// # async fn main() {
/// #    run();    
/// # }
/// #
/// # #[cfg(not(feature = "tokio-runtime"))]
/// # fn main() {
/// #    run();    
/// # }
/// #
/// # fn run() {
/// # Bastion::init();
/// #
/// Bastion::children(|children| {
///     children.with_exec(|ctx| {
///         async move {
///             let child_id: &BastionId = ctx.current().id();
///             // ...
///             # Ok(())
///         }
///     })
/// }).expect("Couldn't create the children group.");
/// #
/// # Bastion::start();
/// # Bastion::stop();
/// # Bastion::block_until_stopped();
/// # }
/// ```
pub struct BastionId(pub(crate) Uuid);

#[derive(Debug)]
/// A child's execution context, allowing its [`with_exec`] future
/// to receive messages and access a [`ChildRef`] referencing
/// it, a [`ChildrenRef`] referencing its children group and
/// a [`SupervisorRef`] referencing its supervisor.
///
/// # Example
///
/// ```rust
/// # use bastion::prelude::*;
/// #
/// # #[cfg(feature = "tokio-runtime")]
/// # #[tokio::main]
/// # async fn main() {
/// #    run();    
/// # }
/// #
/// # #[cfg(not(feature = "tokio-runtime"))]
/// # fn main() {
/// #    run();    
/// # }
/// #
/// # fn run() {
/// # Bastion::init();
/// #
/// Bastion::children(|children| {
///     children.with_exec(|ctx: BastionContext| {
///         async move {
///             // Get a `ChildRef` referencing the child executing
///             // this future...
///             let current: &ChildRef = ctx.current();
///             // Get a `ChildrenRef` referencing the children
///             // group of the child executing this future...
///             let parent: &ChildrenRef = ctx.parent();
///             // Try to get a `SupervisorRef` referencing the
///             // supervisor of the child executing this future...
///             let supervisor: Option<&SupervisorRef> = ctx.supervisor();
///             // Note that `supervisor` will be `None` because
///             // this child was created using `Bastion::children`,
///             // which made it supervised by the system supervisor
///             // (which users can't get a reference to).
///
///             // Try to receive a message...
///             let opt_msg: Option<SignedMessage> = ctx.try_recv().await;
///             // Wait for a message to be received...
///             let msg: SignedMessage = ctx.recv().await?;
///
///             Ok(())
///         }
///     })
/// }).expect("Couldn't create the children group.");
/// #
/// # Bastion::start();
/// # Bastion::stop();
/// # Bastion::block_until_stopped();
/// # }
/// ```
///
/// [`with_exec`]: crate::children::Children::with_exec
pub struct BastionContext {
    id: BastionId,
    child: ChildRef,
    children: ChildrenRef,
    supervisor: Option<SupervisorRef>,
    state: Arc<Pin<Box<ContextState>>>,
}

#[derive(Debug)]
pub(crate) struct ContextState {
    messages: SegQueue<SignedMessage>,
    #[cfg(feature = "scaling")]
    stats: Arc<AtomicU64>,
    #[cfg(feature = "scaling")]
    actor_stats: Arc<LOTable<BastionId, u32>>,
    data: GlobalState,
}

impl BastionId {
    pub(crate) fn new() -> Self {
        let uuid = Uuid::new_v4();

        BastionId(uuid)
    }
}

impl BastionContext {
    pub(crate) fn new(
        id: BastionId,
        child: ChildRef,
        children: ChildrenRef,
        supervisor: Option<SupervisorRef>,
        state: Arc<Pin<Box<ContextState>>>,
    ) -> Self {
        debug!("BastionContext({}): Creating.", id);
        BastionContext {
            id,
            child,
            children,
            supervisor,
            state,
        }
    }

    /// Returns a [`ChildRef`] referencing the children group's
    /// element that is linked to this `BastionContext`.
    ///
    /// # Example
    ///
    /// ```rust
    /// # use bastion::prelude::*;
    /// #
    /// # #[cfg(feature = "tokio-runtime")]
    /// # #[tokio::main]
    /// # async fn main() {
    /// #    run();    
    /// # }
    /// #
    /// # #[cfg(not(feature = "tokio-runtime"))]
    /// # fn main() {
    /// #    run();    
    /// # }
    /// #
    /// # fn run() {
    /// # Bastion::init();
    /// #
    /// Bastion::children(|children| {
    ///     children.with_exec(|ctx: BastionContext| {
    ///         async move {
    ///             let current: &ChildRef = ctx.current();
    ///             // Stop or kill the current element (note that this will
    ///             // only take effect after this future becomes "pending")...
    ///
    ///             Ok(())
    ///         }
    ///     })
    /// }).expect("Couldn't create the children group.");
    /// #
    /// # Bastion::start();
    /// # Bastion::stop();
    /// # Bastion::block_until_stopped();
    /// # }
    /// ```
    pub fn current(&self) -> &ChildRef {
        &self.child
    }

    /// Returns a [`ChildrenRef`] referencing the children group
    /// of the element that is linked to this `BastionContext`.
    ///
    /// # Example
    ///
    /// ```rust
    /// # use bastion::prelude::*;
    /// #
    /// # #[cfg(feature = "tokio-runtime")]
    /// # #[tokio::main]
    /// # async fn main() {
    /// #    run();    
    /// # }
    /// #
    /// # #[cfg(not(feature = "tokio-runtime"))]
    /// # fn main() {
    /// #    run();    
    /// # }
    /// #
    /// # fn run() {
    /// # Bastion::init();
    /// #
    /// Bastion::children(|children| {
    ///     children.with_exec(|ctx: BastionContext| {
    ///         async move {
    ///             let parent: &ChildrenRef = ctx.parent();
    ///             // Get the other elements of the group, broadcast message,
    ///             // or stop or kill the children group...
    ///
    ///             Ok(())
    ///         }
    ///     })
    /// }).expect("Couldn't create the children group.");
    /// #
    /// # Bastion::start();
    /// # Bastion::stop();
    /// # Bastion::block_until_stopped();
    /// # }
    /// ```
    pub fn parent(&self) -> &ChildrenRef {
        &self.children
    }

    /// Returns a [`SupervisorRef`] referencing the supervisor
    /// that supervises the element that is linked to this
    /// `BastionContext` if it isn't the system supervisor
    /// (ie. if the children group wasn't created using
    /// [`Bastion::children`]).
    ///
    /// # Example
    ///
    /// ```rust
    /// # use bastion::prelude::*;
    /// #
    /// # #[cfg(feature = "tokio-runtime")]
    /// # #[tokio::main]
    /// # async fn main() {
    /// #    run();    
    /// # }
    /// #
    /// # #[cfg(not(feature = "tokio-runtime"))]
    /// # fn main() {
    /// #    run();    
    /// # }
    /// #
    /// # fn run() {
    /// # Bastion::init();
    /// #
    /// // When calling the method from a children group supervised
    /// // by a supervisor created by the user...
    /// Bastion::supervisor(|sp| {
    ///     sp.children(|children| {
    ///         children.with_exec(|ctx: BastionContext| {
    ///             async move {
    ///                 // ...the method will return a SupervisorRef referencing the
    ///                 // user-created supervisor...
    ///                 let supervisor: Option<&SupervisorRef> = ctx.supervisor(); // Some
    ///                 assert!(supervisor.is_some());
    ///
    ///                 Ok(())
    ///             }
    ///         })
    ///     })
    /// }).expect("Couldn't create the supervisor.");
    ///
    /// // When calling the method from a children group supervised
    /// // by the system's supervisor...
    /// Bastion::children(|children| {
    ///     children.with_exec(|ctx: BastionContext| {
    ///         async move {
    ///             // ...the method won't return a SupervisorRef...
    ///             let supervisor: Option<&SupervisorRef> = ctx.supervisor(); // None
    ///             assert!(supervisor.is_none());
    ///
    ///             Ok(())
    ///         }
    ///     })
    /// }).expect("Couldn't create the children group.");
    /// #
    /// # Bastion::start();
    /// # Bastion::stop();
    /// # Bastion::block_until_stopped();
    /// # }
    /// ```
    ///
    /// [`Bastion::children`]: struct.Bastion.html#method.children
    pub fn supervisor(&self) -> Option<&SupervisorRef> {
        self.supervisor.as_ref()
    }

    /// Tries to retrieve asynchronously a message received by
    /// the element this `BastionContext` is linked to.
    ///
    /// If you need to wait (always asynchronously) until at
    /// least one message can be retrieved, use [`recv`] instead.
    ///    
    /// If you want to wait for a certain amount of time before bailing out
    /// use [`try_recv_timeout`] instead.
    ///
    /// This method returns [`SignedMessage`] if a message was available, or
    /// `None` otherwise.
    ///
    /// # Example
    ///
    /// ```rust
    /// # use bastion::prelude::*;
    /// #
    /// # #[cfg(feature = "tokio-runtime")]
    /// # #[tokio::main]
    /// # async fn main() {
    /// #    run();    
    /// # }
    /// #
    /// # #[cfg(not(feature = "tokio-runtime"))]
    /// # fn main() {
    /// #    run();    
    /// # }
    /// #
    /// # fn run() {
    /// # Bastion::init();
    /// #
    /// Bastion::children(|children| {
    ///     children.with_exec(|ctx: BastionContext| {
    ///         async move {
    ///             let opt_msg: Option<SignedMessage> = ctx.try_recv().await;
    ///             // If a message was received by the element, `opt_msg` will
    ///             // be `Some(Msg)`, otherwise it will be `None`.
    ///
    ///             Ok(())
    ///         }
    ///     })
    /// }).expect("Couldn't create the children group.");
    /// #
    /// # Bastion::start();
    /// # Bastion::stop();
    /// # Bastion::block_until_stopped();
    /// # }
    /// ```
    ///
    /// [`recv`]: Self::method.recv
    /// [`try_recv_timeout`]: Self::method.try_recv_timeout
    pub async fn try_recv(&self) -> Option<SignedMessage> {
        // We want to let a tick pass
        // otherwise guard will never contain anything.
        Delay::new(Duration::from_millis(0)).await;

        trace!("BastionContext({}): Trying to receive message.", self.id);

        if let Some(msg) = self.state.pop_message() {
            trace!("BastionContext({}): Received message: {:?}", self.id, msg);
            Some(msg)
        } else {
            trace!("BastionContext({}): Received no message.", self.id);
            None
        }
    }

    /// Retrieves asynchronously a message received by the element
    /// this `BastionContext` is linked to and waits (always
    /// asynchronously) for one if none has been received yet.
    ///
    /// If you don't need to wait until at least one message
    /// can be retrieved, use [`try_recv`] instead.
    ///    
    /// If you want to wait for a certain amount of time before bailing out
    /// use [`try_recv_timeout`] instead.
    ///
    /// This method returns [`SignedMessage`] if it succeeded, or `Err(())`
    /// otherwise.
    ///
    /// # Example
    ///
    /// ```rust
    /// # use bastion::prelude::*;
    /// #
    /// # #[cfg(feature = "tokio-runtime")]
    /// # #[tokio::main]
    /// # async fn main() {
    /// #    run();    
    /// # }
    /// #
    /// # #[cfg(not(feature = "tokio-runtime"))]
    /// # fn main() {
    /// #    run();    
    /// # }
    /// #
    /// # fn run() {
    /// # Bastion::init();
    /// #
    /// Bastion::children(|children| {
    ///     children.with_exec(|ctx: BastionContext| {
    ///         async move {
    ///             // This will block until a message has been received...
    ///             let msg: SignedMessage = ctx.recv().await?;
    ///
    ///             Ok(())
    ///         }
    ///     })
    /// }).expect("Couldn't create the children group.");
    /// #
    /// # Bastion::start();
    /// # Bastion::stop();
    /// # Bastion::block_until_stopped();
    /// # }
    /// ```
    ///
    /// [`try_recv`]: Self::try_recv
    /// [`try_recv_timeout`]: Self::try_recv_timeout
    pub async fn recv(&self) -> Result<SignedMessage, ()> {
        debug!("BastionContext({}): Waiting to receive message.", self.id);
        loop {
            if let Some(msg) = self.state.pop_message() {
                trace!("BastionContext({}): Received message: {:?}", self.id, msg);
                return Ok(msg);
            }
            pending!();
        }
    }

    /// Retrieves asynchronously a message received by the element
    /// this `BastionContext` is linked to and waits until `timeout` (always
    /// asynchronously) for one if none has been received yet.
    ///
    /// If you want to wait for ever until at least one message
    /// can be retrieved, use [`recv`] instead.
    ///
    /// If you don't need to wait until at least one message
    /// can be retrieved, use [`try_recv`] instead.
    ///
    /// This method returns [`SignedMessage`] if it succeeded, or `Err(TimeoutError)`
    /// otherwise.
    ///
    /// # Example
    ///
    /// ```rust
    /// # use bastion::prelude::*;
    /// # use std::time::Duration;
    /// #
    /// # #[cfg(feature = "tokio-runtime")]
    /// # #[tokio::main]
    /// # async fn main() {
    /// #    run();    
    /// # }
    /// #
    /// # #[cfg(not(feature = "tokio-runtime"))]
    /// # fn main() {
    /// #    run();    
    /// # }
    /// #
    /// # fn run() {
    /// # Bastion::init();
    /// #
    /// Bastion::children(|children| {
    ///     children.with_exec(|ctx: BastionContext| {
    ///         async move {
    ///             // This will block until a message has been received...
    ///             let timeout = Duration::from_millis(10);
    ///             let msg: SignedMessage = ctx.try_recv_timeout(timeout).await.map_err(|e| {
    ///                if let ReceiveError::Timeout(duration) = e {
    ///                    // Timeout happened      
    ///                }
    ///             })?;
    ///
    ///             Ok(())
    ///         }
    ///     })
    /// }).expect("Couldn't create the children group.");
    /// #
    /// # Bastion::start();
    /// # Bastion::stop();
    /// # Bastion::block_until_stopped();
    /// # }
    /// ```
    ///
    /// [`recv`]: Self::recv
    /// [`try_recv`]: Self::try_recv
    /// [`SignedMessage`]: .crate::enveloppe::SignedMessage
    pub async fn try_recv_timeout(&self, timeout: Duration) -> Result<SignedMessage, ReceiveError> {
        debug!(
            "BastionContext({}): Waiting to receive message within {} milliseconds.",
            self.id,
            timeout.as_millis()
        );
        futures::select! {
            message = self.recv().fuse() => {
                message.map_err(|_| ReceiveError::Other)
            },
            _duration = Delay::new(timeout).fuse() => {
                Err(ReceiveError::Timeout(timeout))
            }
        }
    }

    /// Returns [`RefAddr`] of the current `BastionContext`
    ///
    /// # Example
    ///
    /// ```rust
    /// # use bastion::prelude::*;
    /// #
    /// # #[cfg(feature = "tokio-runtime")]
    /// # #[tokio::main]
    /// # async fn main() {
    /// #    run();    
    /// # }
    /// #
    /// # #[cfg(not(feature = "tokio-runtime"))]
    /// # fn main() {
    /// #    run();    
    /// # }
    /// #
    /// # fn run() {
    /// # Bastion::init();
    /// #
    ///
    /// Bastion::children(|children| {
    ///     children.with_exec(|ctx: BastionContext| {
    ///         async move {
    ///             ctx.tell(&ctx.signature(), "Hello to myself");
    ///
    ///             # Bastion::stop();
    ///             Ok(())
    ///         }
    ///     })
    /// }).expect("Couldn't create the children group.");
    /// #
    /// # Bastion::start();
    /// # Bastion::block_until_stopped();
    /// # }
    /// ```
    ///
    // TODO(scrabsha): should we link to Answer or to RefAddr?
    // [`RefAddr`]: /prelude/struct.Answer.html
    pub fn signature(&self) -> RefAddr {
        RefAddr::new(
            self.current().path().clone(),
            self.current().sender().clone(),
        )
    }

    /// Sends a message to the specified [`RefAddr`]
    ///
    /// # Arguments
    ///
    /// * `to` – the [`RefAddr`] to send the message to
    /// * `msg` – The actual message to send
    ///
    /// # Example
    ///
    /// ```rust
    /// # use bastion::prelude::*;
    /// #
    /// # #[cfg(feature = "tokio-runtime")]
    /// # #[tokio::main]
    /// # async fn main() {
    /// #    run();    
    /// # }
    /// #
    /// # #[cfg(not(feature = "tokio-runtime"))]
    /// # fn main() {
    /// #    run();    
    /// # }
    /// #
    /// # fn run() {
    /// # Bastion::init();
    /// #
    /// Bastion::children(|children| {
    ///     children.with_exec(|ctx: BastionContext| {
    ///         async move {
    ///             // Wait for a message to be received...
    ///             let smsg: SignedMessage = ctx.recv().await?;
    ///             // Obtain address of this message sender...
    ///             let sender_addr = smsg.signature();
    ///             // And send something back
    ///             ctx.tell(&sender_addr, "Ack").expect("Unable to acknowledge");
    ///             Ok(())
    ///         }
    ///     })
    /// }).expect("Couldn't create the children group.");
    /// #
    /// # Bastion::start();
    /// # Bastion::stop();
    /// # Bastion::block_until_stopped();
    /// # }
    /// ```
    pub fn tell<M: Message>(&self, to: &RefAddr, msg: M) -> Result<(), M> {
        debug!(
            "{:?}: Telling message: {:?} to: {:?}",
            self.current().path(),
            msg,
            to.path()
        );
        let msg = BastionMessage::tell(msg);
        let env = Envelope::new_with_sign(msg, self.signature());
        // FIXME: panics?
        to.sender()
            .unbounded_send(env)
            .map_err(|err| err.into_inner().into_msg().unwrap())
    }

    /// Sends a message from behalf of current context to the addr,
    /// allowing to addr owner answer.
    ///
    /// This method returns [`Answer`] if it succeeded, or `Err(msg)`
    /// otherwise.
    ///
    /// # Argument
    ///
    /// * `msg` - The message to send.
    ///
    /// # Example
    ///
    /// ```
    /// # use bastion::prelude::*;
    /// #
    /// # #[cfg(feature = "tokio-runtime")]
    /// # #[tokio::main]
    /// # async fn main() {
    /// #    run();    
    /// # }
    /// #
    /// # #[cfg(not(feature = "tokio-runtime"))]
    /// # fn main() {
    /// #    run();    
    /// # }
    /// #
    /// # fn run() {
    ///     # Bastion::init();
    /// // The message that will be "asked"...
    /// const ASK_MSG: &'static str = "A message containing data (ask).";
    /// // The message the will be "answered"...
    /// const ANSWER_MSG: &'static str = "A message containing data (answer).";
    ///
    ///     # let children_ref =
    /// // Create a new child...
    /// Bastion::children(|children| {
    ///     children.with_exec(|ctx: BastionContext| {
    ///         async move {
    ///             // ...which will receive the message asked...
    ///             msg! { ctx.recv().await?,
    ///                 msg: &'static str =!> {
    ///                     assert_eq!(msg, ASK_MSG);
    ///                     // Handle the message...
    ///
    ///                     // ...and eventually answer to it...
    ///                     answer!(ctx, ANSWER_MSG);
    ///                 };
    ///                 // This won't happen because this example
    ///                 // only "asks" a `&'static str`...
    ///                 _: _ => ();
    ///             }
    ///
    ///             Ok(())
    ///         }
    ///     })
    /// }).expect("Couldn't create the children group.");
    ///
    ///     # Bastion::children(|children| {
    ///         # children.with_exec(move |ctx: BastionContext| {
    ///             # let child_ref = children_ref.elems()[0].clone();
    ///             # async move {
    /// // Later, the message is "asked" to the child...
    /// let answer: Answer = ctx.ask(&child_ref.addr(), ASK_MSG).expect("Couldn't send the message.");
    ///
    /// // ...and the child's answer is received...
    /// msg! { answer.await.expect("Couldn't receive the answer."),
    ///     msg: &'static str => {
    ///         assert_eq!(msg, ANSWER_MSG);
    ///         // Handle the answer...
    ///     };
    ///     // This won't happen because this example
    ///     // only answers a `&'static str`...
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
    pub fn ask<M: Message>(&self, to: &RefAddr, msg: M) -> Result<Answer, M> {
        debug!(
            "{:?}: Asking message: {:?} to: {:?}",
            self.current().path(),
            msg,
            to
        );
        let (msg, answer) = BastionMessage::ask(msg, self.signature());
        let env = Envelope::new_with_sign(msg, self.signature());
        // FIXME: panics?
        to.sender()
            .unbounded_send(env)
            .map_err(|err| err.into_inner().into_msg().unwrap())?;

        Ok(answer)
    }

    /// Sends the notification to each declared dispatcher of the actor.
    ///
    /// # Argument
    ///
    /// * `dispatchers` - Vector of dispatcher names to which need to
    /// deliver a notification.
    /// * `notification_type` - The type of the notification to send.
    ///
    pub fn notify(&self, dispatchers: &[DispatcherType], notification_type: NotificationType) {
        let global_dispatcher = SYSTEM.dispatcher();
        let from_actor = self.current();
        global_dispatcher.notify(from_actor, dispatchers, notification_type);
    }

    /// Sends the broadcasted message to the target group(s).
    ///
    /// # Argument
    ///
    /// * `target` - Defines the message receivers in according with
    /// the [`BroadcastTarget`] value.
    /// * `message` - The broadcasted message.
    ///
    pub fn broadcast_message<M: Message>(&self, target: BroadcastTarget, message: M) {
        let msg = Arc::new(SignedMessage {
            msg: Msg::broadcast(message),
            sign: self.signature(),
        });

        let global_dispatcher = SYSTEM.dispatcher();
        global_dispatcher.broadcast_message(target, &msg);
    }

    // TODO(scrabsha): add doc
    pub fn read_data<T: Send + Sync + 'static, O>(&self, f: impl FnOnce(Option<&T>) -> O) -> O {
        self.state.data.read(|t| f(t))
    }
}

impl ContextState {
    pub(crate) fn with_globals(globals: GlobalState) -> Self {
        ContextState {
            messages: SegQueue::new(),
            #[cfg(feature = "scaling")]
            stats: Arc::new(AtomicU64::new(0)),
            #[cfg(feature = "scaling")]
            actor_stats: Arc::new(LOTable::new()),
            data: globals,
        }
    }

    #[cfg(feature = "scaling")]
    pub(crate) fn set_stats(&mut self, stats: Arc<AtomicU64>) {
        self.stats = stats;
    }

    #[cfg(feature = "scaling")]
    pub(crate) fn set_actor_stats(&mut self, actor_stats: Arc<LOTable<BastionId, u32>>) {
        self.actor_stats = actor_stats;
    }

    #[cfg(feature = "scaling")]
    pub(crate) fn stats(&self) -> Arc<AtomicU64> {
        self.stats.clone()
    }

    #[cfg(feature = "scaling")]
    pub(crate) fn actor_stats(&self) -> Arc<LOTable<BastionId, u32>> {
        self.actor_stats.clone()
    }

    pub(crate) fn push_message(&self, msg: Msg, sign: RefAddr) {
        self.messages.push(SignedMessage::new(msg, sign))
    }

    pub(crate) fn pop_message(&self) -> Option<SignedMessage> {
        self.messages.pop()
    }

    #[cfg(feature = "scaling")]
    pub(crate) fn mailbox_size(&self) -> u32 {
        self.messages.len() as _
    }

    pub fn state(&self) -> GlobalState {
        SYSTEM.state()
    }
}

impl Display for BastionId {
    fn fmt(&self, fmt: &mut Formatter) -> fmt::Result {
        self.0.fmt(fmt)
    }
}

#[cfg(test)]
mod context_tests {
    use super::*;
    use crate::prelude::*;
    use crate::Bastion;
    use std::panic;

    #[cfg(feature = "tokio-runtime")]
    mod tokio_tests {
        #[tokio::test]
        async fn test_context() {
            super::test_context()
        }
    }

    #[cfg(not(feature = "tokio-runtime"))]
    mod no_tokio_tests {
        #[test]
        fn test_context() {
            super::test_context()
        }
    }

    fn test_context() {
        Bastion::init();
        Bastion::start();

        test_recv();
        test_try_recv();
        test_try_recv_fail();
        test_try_recv_timeout();
        test_try_recv_timeout_fail();
    }

    fn test_recv() {
        let children = Bastion::children(|children| {
            children.with_exec(|ctx: BastionContext| async move {
                msg! { ctx.recv().await?,
                    ref msg: &'static str => {
                        assert_eq!(msg, &"test recv");
                    };
                    msg: _ => { panic!("didn't receive the expected message {:?}", msg);};
                }
                Ok(())
            })
        })
        .expect("Couldn't create the children group.");

        children
            .broadcast("test recv")
            .expect("couldn't send message");
    }

    fn test_try_recv() {
        let children = Bastion::children(|children| {
            children.with_exec(|ctx: BastionContext| async move {
                // make sure the message has been sent
                Delay::new(std::time::Duration::from_millis(1)).await;
                msg! { ctx.try_recv().await.expect("no message"),
                    ref msg: &'static str => {
                        assert_eq!(msg, &"test try recv");
                    };
                    _: _ => { panic!("didn't receive the expected message");};
                }
                Ok(())
            })
        })
        .expect("Couldn't create the children group.");

        children
            .broadcast("test try recv")
            .expect("couldn't send message");
    }

    fn test_try_recv_fail() {
        Bastion::children(|children| {
            children.with_exec(|ctx: BastionContext| async move {
                assert!(ctx.try_recv().await.is_none());
                Ok(())
            })
        })
        .expect("Couldn't create the children group.");

        // Not sending any message
    }

    fn test_try_recv_timeout() {
        let children =
        Bastion::children(|children| {
            children.with_exec(|ctx: BastionContext| async move {
                msg! { ctx.try_recv_timeout(std::time::Duration::from_millis(5)).await.expect("recv_timeout failed"),
                    ref msg: &'static str => {
                        assert_eq!(msg, &"test recv timeout");
                    };
                    _: _ => { panic!("didn't receive the expected message");};
                }
                Ok(())
            })
        })
        .expect("Couldn't create the children group.");

        children
            .broadcast("test recv timeout")
            .expect("couldn't send message");
    }

    fn test_try_recv_timeout_fail() {
        let children = Bastion::children(|children| {
            children.with_exec(|ctx: BastionContext| async move {
                assert!(ctx
                    .try_recv_timeout(std::time::Duration::from_millis(1))
                    .await
                    .is_err());
                Ok(())
            })
        })
        .expect("Couldn't create the children group.");

        // Triggering the timeout
        run!(async { Delay::new(std::time::Duration::from_millis(2)).await });

        // The child panicked, but we should still be able to send things to it
        children.broadcast("test recv timeout").unwrap();
    }
}
