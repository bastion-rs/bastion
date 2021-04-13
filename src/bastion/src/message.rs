//!
//! Dynamic dispatch oriented messaging system
//!
//! This system allows:
//! * Generic communication between mailboxes.
//! * All message communication relies on at-most-once delivery guarantee.
//! * Messages are not guaranteed to be ordered, all message's order is causal.
//!
use crate::callbacks::CallbackType;
use crate::children::Children;
use crate::context::{BastionId, ContextState};
use crate::envelope::{RefAddr, SignedMessage};
use crate::supervisor::{SupervisionStrategy, Supervisor};

use futures::channel::oneshot::{self, Receiver};
use lightproc::proc_state::State;
use std::any::{type_name, Any};
use std::fmt::Debug;
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use std::task::{Context, Poll};
use tracing::{debug, trace};

/// A trait that any message sent needs to implement (it is
/// already automatically implemented but forces message to
/// implement the following traits: [`Any`], [`Send`],
/// [`Sync`] and [`Debug`]).
///
/// [`Any`]: std::any::Any
/// [`Send`]: std::marker::Send
/// [`Sync`]: std::marker::Sync
/// [`Debug`]: std::fmt::Debug
pub trait Message: Any + Send + Sync + Debug {}
impl<T> Message for T where T: Any + Send + Sync + Debug {}

/// Allows to respond to questions.
///
/// This type features the [`respond`] method, that allows to respond to a
/// question.
///
/// [`respond`]: #method.respond
#[derive(Debug)]
pub struct AnswerSender(oneshot::Sender<SignedMessage>, RefAddr);

#[derive(Debug)]
/// A [`Future`] returned when successfully "asking" a
/// message using [`ChildRef::ask_anonymously`] and which resolves to
/// a `Result<Msg, ()>` where the [`Msg`] is the message
/// answered by the child (see the [`msg!`] macro for more
/// information).
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
///
/// [`Future`]: std::future::Future
/// [`ChildRef::ask_anonymously`]: crate::child_ref::ChildRef::ask_anonymously
pub struct Answer(Receiver<SignedMessage>);

#[derive(Debug)]
/// A message returned by [`BastionContext::recv`] or
/// [`BastionContext::try_recv`] that should be passed to the
/// [`msg!`] macro to try to match what its real type is.
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
///     # Bastion::init();
/// Bastion::children(|children| {
///     children.with_exec(|ctx: BastionContext| {
///         async move {
///             loop {
///                 let msg: SignedMessage = ctx.recv().await?;
///                 msg! { msg,
///                     // We match a broadcasted `&'static str`s...
///                     ref msg: &'static str => {
///                         // Note that `msg` will actually be a `&&'static str`.
///                         assert_eq!(msg, &"A message containing data.");
///
///                         // Handle the message...
///                     };
///                     // We match a `&'static str`s "told" to this child...
///                     msg: &'static str => {
///                         assert_eq!(msg, "A message containing data.");
///                         // Handle the message...
///
///                         // get message signature
///                         let sign = signature!();
///                         ctx.tell(&sign, "A message containing reply").unwrap();
///                     };
///                     // We match a `&'static str`s "asked" to this child...
///                     msg: &'static str =!> {
///                         assert_eq!(msg, "A message containing data.");
///                         // Handle the message...
///
///                         // ...and eventually answer to it...
///                         answer!(ctx, "An answer message containing data.");
///                     };
///                     // We match a message that wasn't previously matched...
///                     _: _ => ();
///                 }
///             }
///         }
///     })
/// }).expect("Couldn't start the children group.");
///     #
///     # Bastion::start();
///     # Bastion::stop();
///     # Bastion::block_until_stopped();
/// # }
/// ```
///
/// [`BastionContext::recv`]: crate::context::BastionContext::recv
/// [`BastionContext::try_recv`]: crate::context::BastionContext::try_recv
pub struct Msg(MsgInner);

#[derive(Debug)]
enum MsgInner {
    Broadcast(Arc<dyn Any + Send + Sync + 'static>),
    Tell(Box<dyn Any + Send + Sync + 'static>),
    Ask {
        msg: Box<dyn Any + Send + Sync + 'static>,
        sender: Option<AnswerSender>,
    },
}

#[derive(Debug)]
pub(crate) enum BastionMessage {
    Start,
    Stop,
    Kill,
    Deploy(Box<Deployment>),
    Prune {
        id: BastionId,
    },
    SuperviseWith(SupervisionStrategy),
    ApplyCallback(CallbackType),
    InstantiatedChild {
        parent_id: BastionId,
        child_id: BastionId,
        state: Arc<Pin<Box<ContextState>>>,
    },
    Message(Msg),
    RestartRequired {
        id: BastionId,
        parent_id: BastionId,
    },
    FinishedChild {
        id: BastionId,
        parent_id: BastionId,
    },
    RestartSubtree,
    RestoreChild {
        id: BastionId,
        state: Arc<Pin<Box<ContextState>>>,
    },
    DropChild {
        id: BastionId,
    },
    SetState {
        state: Arc<Pin<Box<ContextState>>>,
    },
    Stopped {
        id: BastionId,
    },
    Faulted {
        id: BastionId,
    },
    Heartbeat,
}

#[derive(Debug)]
pub(crate) enum Deployment {
    Supervisor(Supervisor),
    Children(Children),
}

impl AnswerSender {
    /// Sends data back to the original sender.
    ///
    /// Returns  `Ok` if the data was sent successfully, otherwise returns the
    /// original data.
    pub fn reply<M: Message>(self, msg: M) -> Result<(), M> {
        debug!("{:?}: Sending answer: {:?}", self, msg);
        let msg = Msg::tell(msg);
        trace!("{:?}: Sending message: {:?}", self, msg);

        let AnswerSender(sender, sign) = self;
        sender
            .send(SignedMessage::new(msg, sign))
            .map_err(|smsg| smsg.msg.try_unwrap().unwrap())
    }
}

impl Msg {
    pub(crate) fn broadcast<M: Message>(msg: M) -> Self {
        let inner = MsgInner::Broadcast(Arc::new(msg));
        Msg(inner)
    }

    pub(crate) fn tell<M: Message>(msg: M) -> Self {
        let inner = MsgInner::Tell(Box::new(msg));
        Msg(inner)
    }

    pub(crate) fn ask<M: Message>(msg: M, sign: RefAddr) -> (Self, Answer) {
        let msg = Box::new(msg);
        let (sender, recver) = oneshot::channel();
        let sender = AnswerSender(sender, sign);
        let answer = Answer(recver);

        let sender = Some(sender);
        let inner = MsgInner::Ask { msg, sender };

        (Msg(inner), answer)
    }

    #[doc(hidden)]
    pub fn is_broadcast(&self) -> bool {
        matches!(self.0, MsgInner::Broadcast(_))
    }

    #[doc(hidden)]
    pub fn is_tell(&self) -> bool {
        matches!(self.0, MsgInner::Tell(_))
    }

    #[doc(hidden)]
    pub fn is_ask(&self) -> bool {
        matches!(self.0, MsgInner::Ask { .. })
    }

    #[doc(hidden)]
    pub fn take_sender(&mut self) -> Option<AnswerSender> {
        debug!("{:?}: Taking sender.", self);
        if let MsgInner::Ask { sender, .. } = &mut self.0 {
            sender.take()
        } else {
            None
        }
    }

    #[doc(hidden)]
    pub fn is<M: Message>(&self) -> bool {
        match &self.0 {
            MsgInner::Tell(msg) => msg.is::<M>(),
            MsgInner::Ask { msg, .. } => msg.is::<M>(),
            MsgInner::Broadcast(msg) => msg.is::<M>(),
        }
    }

    #[doc(hidden)]
    pub fn downcast<M: Message>(self) -> Result<M, Self> {
        trace!("{:?}: Downcasting to {}.", self, type_name::<M>());
        match self.0 {
            MsgInner::Tell(msg) => {
                if msg.is::<M>() {
                    let msg: Box<dyn Any + 'static> = msg;
                    Ok(*msg.downcast().unwrap())
                } else {
                    let inner = MsgInner::Tell(msg);
                    Err(Msg(inner))
                }
            }
            MsgInner::Ask { msg, sender } => {
                if msg.is::<M>() {
                    let msg: Box<dyn Any + 'static> = msg;
                    Ok(*msg.downcast().unwrap())
                } else {
                    let inner = MsgInner::Ask { msg, sender };
                    Err(Msg(inner))
                }
            }
            _ => Err(self),
        }
    }

    #[doc(hidden)]
    pub fn downcast_ref<M: Message>(&self) -> Option<Arc<M>> {
        trace!("{:?}: Downcasting to ref of {}.", self, type_name::<M>());
        if let MsgInner::Broadcast(msg) = &self.0 {
            if msg.is::<M>() {
                return Some(msg.clone().downcast::<M>().unwrap());
            }
        }

        None
    }

    pub(crate) fn try_clone(&self) -> Option<Self> {
        trace!("{:?}: Trying to clone.", self);
        if let MsgInner::Broadcast(msg) = &self.0 {
            let inner = MsgInner::Broadcast(msg.clone());
            Some(Msg(inner))
        } else {
            None
        }
    }

    pub(crate) fn try_unwrap<M: Message>(self) -> Result<M, Self> {
        debug!("{:?}: Trying to unwrap.", self);
        if let MsgInner::Broadcast(msg) = self.0 {
            match msg.downcast() {
                Ok(msg) => match Arc::try_unwrap(msg) {
                    Ok(msg) => Ok(msg),
                    Err(msg) => {
                        let inner = MsgInner::Broadcast(msg);
                        Err(Msg(inner))
                    }
                },
                Err(msg) => {
                    let inner = MsgInner::Broadcast(msg);
                    Err(Msg(inner))
                }
            }
        } else {
            self.downcast()
        }
    }
}

impl AsRef<dyn Any> for Msg {
    fn as_ref(&self) -> &dyn Any {
        match &self.0 {
            MsgInner::Broadcast(msg) => msg.as_ref(),
            MsgInner::Tell(msg) => msg.as_ref(),
            MsgInner::Ask { msg, .. } => msg.as_ref(),
        }
    }
}

impl BastionMessage {
    pub(crate) fn start() -> Self {
        BastionMessage::Start
    }

    pub(crate) fn stop() -> Self {
        BastionMessage::Stop
    }

    pub(crate) fn kill() -> Self {
        BastionMessage::Kill
    }

    pub(crate) fn deploy_supervisor(supervisor: Supervisor) -> Self {
        let deployment = Deployment::Supervisor(supervisor);

        BastionMessage::Deploy(deployment.into())
    }

    pub(crate) fn deploy_children(children: Children) -> Self {
        let deployment = Deployment::Children(children);

        BastionMessage::Deploy(deployment.into())
    }

    pub(crate) fn prune(id: BastionId) -> Self {
        BastionMessage::Prune { id }
    }

    pub(crate) fn supervise_with(strategy: SupervisionStrategy) -> Self {
        BastionMessage::SuperviseWith(strategy)
    }

    pub(crate) fn apply_callback(callback_type: CallbackType) -> Self {
        BastionMessage::ApplyCallback(callback_type)
    }

    pub(crate) fn instantiated_child(
        parent_id: BastionId,
        child_id: BastionId,
        state: Arc<Pin<Box<ContextState>>>,
    ) -> Self {
        BastionMessage::InstantiatedChild {
            parent_id,
            child_id,
            state,
        }
    }

    pub(crate) fn broadcast<M: Message>(msg: M) -> Self {
        let msg = Msg::broadcast(msg);
        BastionMessage::Message(msg)
    }

    pub(crate) fn tell<M: Message>(msg: M) -> Self {
        let msg = Msg::tell(msg);
        BastionMessage::Message(msg)
    }

    pub(crate) fn ask<M: Message>(msg: M, sign: RefAddr) -> (Self, Answer) {
        let (msg, answer) = Msg::ask(msg, sign);
        (BastionMessage::Message(msg), answer)
    }

    pub(crate) fn restart_required(id: BastionId, parent_id: BastionId) -> Self {
        BastionMessage::RestartRequired { id, parent_id }
    }

    pub(crate) fn finished_child(id: BastionId, parent_id: BastionId) -> Self {
        BastionMessage::FinishedChild { id, parent_id }
    }

    pub(crate) fn restart_subtree() -> Self {
        BastionMessage::RestartSubtree
    }

    pub(crate) fn restore_child(id: BastionId, state: Arc<Pin<Box<ContextState>>>) -> Self {
        BastionMessage::RestoreChild { id, state }
    }

    pub(crate) fn drop_child(id: BastionId) -> Self {
        BastionMessage::DropChild { id }
    }

    pub(crate) fn set_state(state: Arc<Pin<Box<ContextState>>>) -> Self {
        BastionMessage::SetState { state }
    }

    pub(crate) fn stopped(id: BastionId) -> Self {
        BastionMessage::Stopped { id }
    }

    pub(crate) fn faulted(id: BastionId) -> Self {
        BastionMessage::Faulted { id }
    }

    pub(crate) fn heartbeat() -> Self {
        BastionMessage::Heartbeat
    }

    pub(crate) fn try_clone(&self) -> Option<Self> {
        trace!("{:?}: Trying to clone.", self);
        let clone = match self {
            BastionMessage::Start => BastionMessage::start(),
            BastionMessage::Stop => BastionMessage::stop(),
            BastionMessage::Kill => BastionMessage::kill(),
            // FIXME
            BastionMessage::Deploy(_) => unimplemented!(),
            BastionMessage::Prune { id } => BastionMessage::prune(id.clone()),
            BastionMessage::SuperviseWith(strategy) => {
                BastionMessage::supervise_with(strategy.clone())
            }
            BastionMessage::ApplyCallback(callback_type) => {
                BastionMessage::apply_callback(callback_type.clone())
            }
            BastionMessage::InstantiatedChild {
                parent_id,
                child_id,
                state,
            } => BastionMessage::instantiated_child(
                parent_id.clone(),
                child_id.clone(),
                state.clone(),
            ),
            BastionMessage::Message(msg) => BastionMessage::Message(msg.try_clone()?),
            BastionMessage::RestartRequired { id, parent_id } => {
                BastionMessage::restart_required(id.clone(), parent_id.clone())
            }
            BastionMessage::FinishedChild { id, parent_id } => {
                BastionMessage::finished_child(id.clone(), parent_id.clone())
            }
            BastionMessage::RestartSubtree => BastionMessage::restart_subtree(),
            BastionMessage::RestoreChild { id, state } => {
                BastionMessage::restore_child(id.clone(), state.clone())
            }
            BastionMessage::DropChild { id } => BastionMessage::drop_child(id.clone()),
            BastionMessage::SetState { state } => BastionMessage::set_state(state.clone()),
            BastionMessage::Stopped { id } => BastionMessage::stopped(id.clone()),
            BastionMessage::Faulted { id } => BastionMessage::faulted(id.clone()),
            BastionMessage::Heartbeat => BastionMessage::heartbeat(),
        };

        Some(clone)
    }

    pub(crate) fn into_msg<M: Message>(self) -> Option<M> {
        if let BastionMessage::Message(msg) = self {
            msg.try_unwrap().ok()
        } else {
            None
        }
    }
}

impl Future for Answer {
    type Output = Result<SignedMessage, ()>;

    fn poll(self: Pin<&mut Self>, ctx: &mut Context) -> Poll<Self::Output> {
        debug!("{:?}: Polling.", self);
        Pin::new(&mut self.get_mut().0).poll(ctx).map_err(|_| ())
    }
}

#[macro_export]
/// Matches a [`Msg`] (as returned by [`BastionContext::recv`]
/// or [`BastionContext::try_recv`]) with different types.
///
/// Each case is defined as:
/// - an optional `ref` which will make the case only match
///   if the message was broadcasted
/// - a variable name for the message if it matched this case
/// - a colon
/// - a type that the message must be of to match this case
///   (note that if the message was broadcasted, the actual
///   type of the variable will be a reference to this type)
/// - an arrow (`=>`) with an optional bang (`!`) between
///   the equal and greater-than signs which will make the
///   case only match if the message can be answered
/// - code that will be executed if the case matches
///
/// If the message can be answered (when using `=!>` instead
/// of `=>` as said above), an answer can be sent by passing
/// it to the `answer!` macro that will be generated for this
/// use.
///
/// A default case is required, which is defined in the same
/// way as any other case but with its type set as `_` (note
/// that it doesn't has the optional `ref` or `=!>`).
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
///     # Bastion::init();
/// // The message that will be broadcasted...
/// const BCAST_MSG: &'static str = "A message containing data (broadcast).";
/// // The message that will be "told" to the child...
/// const TELL_MSG: &'static str = "A message containing data (tell).";
/// // The message that will be "asked" to the child...
/// const ASK_MSG: &'static str = "A message containing data (ask).";
///
/// Bastion::children(|children| {
///     children.with_exec(|ctx: BastionContext| {
///         async move {
///             # ctx.tell(&ctx.current().addr(), TELL_MSG).unwrap();
///             # ctx.ask(&ctx.current().addr(), ASK_MSG).unwrap();
///             #
///             loop {
///                 msg! { ctx.recv().await?,
///                     // We match broadcasted `&'static str`s...
///                     ref msg: &'static str => {
///                         // Note that `msg` will actually be a `&&'static str`.
///                         assert_eq!(msg, &BCAST_MSG);
///                         // Handle the message...
///                     };
///                     // We match `&'static str`s "told" to this child...
///                     msg: &'static str => {
///                         assert_eq!(msg, TELL_MSG);
///                         // Handle the message...
///                     };
///                     // We match `&'static str`'s "asked" to this child...
///                     msg: &'static str =!> {
///                         assert_eq!(msg, ASK_MSG);
///                         // Handle the message...
///
///                         // ...and eventually answer to it...
///                         answer!(ctx, "An answer to the message.");
///                     };
///                     // We are only broadcasting, "telling" and "asking" a
///                     // `&'static str` in this example, so we know that this won't
///                     // happen...
///                     _: _ => ();
///                 }
///             }
///         }
///     })
/// }).expect("Couldn't start the children group.");
///     #
///     # Bastion::start();
///     # Bastion::broadcast(BCAST_MSG).unwrap();
///     # Bastion::stop();
///     # Bastion::block_until_stopped();
/// # }
/// ```
///
/// [`BastionContext::recv`]: crate::context::BastionContext::recv
/// [`BastionContext::try_recv`]: crate::context::BastionContext::try_recv
macro_rules! msg {
    ($msg:expr, $($tokens:tt)+) => {
        msg!(@internal $msg, (), (), (), $($tokens)+)
    };

    (@internal
        $msg:expr,
        ($($bvar:ident, $bty:ty, $bhandle:expr,)*),
        ($($tvar:ident, $tty:ty, $thandle:expr,)*),
        ($($avar:ident, $aty:ty, $ahandle:expr,)*),
        ref $var:ident: $ty:ty => $handle:expr;
        $($rest:tt)+
    ) => {
        msg!(@internal $msg,
            ($($bvar, $bty, $bhandle,)* $var, $ty, $handle,),
            ($($tvar, $tty, $thandle,)*),
            ($($avar, $aty, $ahandle,)*),
            $($rest)+
        )
    };

    (@internal
        $msg:expr,
        ($($bvar:ident, $bty:ty, $bhandle:expr,)*),
        ($($tvar:ident, $tty:ty, $thandle:expr,)*),
        ($($avar:ident, $aty:ty, $ahandle:expr,)*),
        $var:ident: $ty:ty => $handle:expr;
        $($rest:tt)+
    ) => {
        msg!(@internal $msg,
            ($($bvar, $bty, $bhandle,)*),
            ($($tvar, $tty, $thandle,)* $var, $ty, $handle,),
            ($($avar, $aty, $ahandle,)*),
            $($rest)+
        )
    };

    (@internal
        $msg:expr,
        ($($bvar:ident, $bty:ty, $bhandle:expr,)*),
        ($($tvar:ident, $tty:ty, $thandle:expr,)*),
        ($($avar:ident, $aty:ty, $ahandle:expr,)*),
        $var:ident: $ty:ty =!> $handle:expr;
        $($rest:tt)+
    ) => {
        msg!(@internal $msg,
            ($($bvar, $bty, $bhandle,)*),
            ($($tvar, $tty, $thandle,)*),
            ($($avar, $aty, $ahandle,)* $var, $ty, $handle,),
            $($rest)+
        )
    };

    (@internal
        $msg:expr,
        ($($bvar:ident, $bty:ty, $bhandle:expr,)*),
        ($($tvar:ident, $tty:ty, $thandle:expr,)*),
        ($($avar:ident, $aty:ty, $ahandle:expr,)*),
        _: _ => $handle:expr;
    ) => {
        msg!(@internal $msg,
            ($($bvar, $bty, $bhandle,)*),
            ($($tvar, $tty, $thandle,)*),
            ($($avar, $aty, $ahandle,)*),
            msg: _ => $handle;
        )
    };

    (@internal
        $msg:expr,
        ($($bvar:ident, $bty:ty, $bhandle:expr,)*),
        ($($tvar:ident, $tty:ty, $thandle:expr,)*),
        ($($avar:ident, $aty:ty, $ahandle:expr,)*),
        $var:ident: _ => $handle:expr;
    ) => { {
        let mut signed = $msg;

        let (mut $var, sign) = signed.extract();

        macro_rules! signature {
            () => {
                sign
            };
        }

        let sender = $var.take_sender();
        if $var.is_broadcast() {
            if false {
                unreachable!();
            }
            $(
                else if $var.is::<$bty>() {
                    let $bvar = &*$var.downcast_ref::<$bty>().unwrap();
                    { $bhandle }
                }
            )*
            else {
                { $handle }
            }
        } else if sender.is_some() {
            let sender = sender.unwrap();

            macro_rules! answer {
                ($ctx:expr, $answer:expr) => {
                    {
                        let sign = $ctx.signature();
                        sender.reply($answer)
                    }
                };
            }

            if false {
                unreachable!();
            }
            $(
                else if $var.is::<$aty>() {
                    let $avar = $var.downcast::<$aty>().unwrap();
                    { $ahandle }
                }
            )*
            else {
                { $handle }
            }
        } else {
            if false {
                unreachable!();
            }
            $(
                else if $var.is::<$tty>() {
                    let $tvar = $var.downcast::<$tty>().unwrap();
                    { $thandle }
                }
            )*
            else {
                { $handle }
            }
        }
    } };
}

#[macro_export]
/// Answers to a given message, with the given answer.
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
///     # Bastion::init();
///     # let children_ref =
/// // Create a new child...
/// Bastion::children(|children| {
///     children.with_exec(|ctx: BastionContext| {
///         async move {
///             let msg = ctx.recv().await?;
///             answer!(msg, "goodbye").unwrap();
///             Ok(())
///         }
///     })
/// }).expect("Couldn't create the children group.");
///
///     # Bastion::children(|children| {
///         # children.with_exec(move |ctx: BastionContext| {
///             # let child_ref = children_ref.elems()[0].clone();
///             # async move {
/// // now you can ask the child, from another children
/// let answer: Answer = ctx.ask(&child_ref.addr(), "hello").expect("Couldn't send the message.");
///
/// msg! { answer.await.expect("Couldn't receive the answer."),
///     msg: &'static str => {
///         assert_eq!(msg, "goodbye");
///     };
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
macro_rules! answer {
    ($msg:expr, $answer:expr) => {{
        let (mut msg, sign) = $msg.extract();
        let sender = msg.take_sender().expect("failed to take render");
        sender.reply($answer)
    }};
}

#[derive(Debug)]
enum MessageHandlerState<O> {
    Matched(O),
    Unmatched(SignedMessage),
}

impl<O> MessageHandlerState<O> {
    fn take_message(self) -> Result<SignedMessage, O> {
        match self {
            MessageHandlerState::Unmatched(msg) => Ok(msg),
            MessageHandlerState::Matched(output) => Err(output),
        }
    }

    fn output_or_else(self, f: impl FnOnce(SignedMessage) -> O) -> O {
        match self {
            MessageHandlerState::Matched(output) => output,
            MessageHandlerState::Unmatched(msg) => f(msg),
        }
    }
}

/// Matches a [`Msg`] (as returned by [`BastionContext::recv`]
/// or [`BastionContext::try_recv`]) with different types.
///
/// This type may replace the [`msg!`] macro in the future.
///
/// The [`new`] function creates a new [`MessageHandler`], which is then
/// matched on with the `on_*` functions.
///
/// There are different kind of messages:
///   - messages that are broadcasted, which can be matched with the
///     [`on_broadcast`] method,
///   - messages that can be responded to, which are matched with the
///     [`on_question`] method,
///   - messages that can not be responded to, which are matched with
///     [`on_tell`],
///   - fallback case, which matches everything, entitled [`on_fallback`].
///
/// The closure passed to the functions described previously must return the
/// same type. This value is retrieved when [`on_fallback`] is invoked.
///
/// Questions can be responded to by calling [`reply`] on the provided
/// sender.
///
/// # Example
///
/// ```rust
/// # use bastion::prelude::*;
/// # use bastion::message::MessageHandler;
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
/// // The message that will be broadcasted...
/// const BCAST_MSG: &'static str = "A message containing data (broadcast).";
/// // The message that will be "told" to the child...
/// const TELL_MSG: &'static str = "A message containing data (tell).";
/// // The message that will be "asked" to the child...
/// const ASK_MSG: &'static str = "A message containing data (ask).";
///
/// Bastion::children(|children| {
///     children.with_exec(|ctx: BastionContext| {
///         async move {
///             # ctx.tell(&ctx.current().addr(), TELL_MSG).unwrap();
///             # ctx.ask(&ctx.current().addr(), ASK_MSG).unwrap();
///             #
///             loop {
///                 MessageHandler::new(ctx.recv().await?)
///                     // We match on broadcasts of &str
///                     .on_broadcast(|msg: &&str, _sender_addr| {
///                         assert_eq!(*msg, BCAST_MSG);
///                         // Handle the message...
///                     })
///                     // We match on messages of &str
///                     .on_tell(|msg: &str, _sender_addr| {
///                         assert_eq!(msg, TELL_MSG);
///                         // Handle the message...
///                     })
///                     // We match on questions of &str
///                     .on_question(|msg: &str, sender| {
///                         assert_eq!(msg, ASK_MSG);
///                         // Handle the message...
///
///                         // ...and eventually answer to it...
///                         sender.reply("An answer to the message.");
///                     })
///                     // We are only broadcasting, "telling" and "asking" a
///                     // `&str` in this example, so we know that this won't
///                     // happen...
///                     .on_fallback(|msg, _sender_addr| ());
///             }
///         }
///     })
/// }).expect("Couldn't start the children group.");
///     #
///     # Bastion::start();
///     # Bastion::broadcast(BCAST_MSG).unwrap();
///     # Bastion::stop();
///     # Bastion::block_until_stopped();
/// # }
/// ```
///
/// [`BastionContext::recv`]: crate::context::BastionContext::recv
/// [`BastionContext::try_recv`]: crate::context::BastionContext::try_recv
/// [`new`]: Self::new
/// [`on_broadcast`]: Self::on_broadcast
/// [`on_question`]: Self::on_question
/// [`on_tell`]: Self::on_tell
/// [`on_fallback`]: Self::on_fallback
/// [`reply`]: AnswerSender::reply
#[derive(Debug)]
pub struct MessageHandler<O> {
    state: MessageHandlerState<O>,
}

impl<O> MessageHandler<O> {
    /// Creates a new [`MessageHandler`] with an incoming message.
    pub fn new(msg: SignedMessage) -> MessageHandler<O> {
        let state = MessageHandlerState::Unmatched(msg);
        MessageHandler { state }
    }

    /// Matches on a question of a specific type.
    ///
    /// This will consume the inner data and call `f` if the contained message
    /// can be replied to.
    pub fn on_question<T, F>(self, f: F) -> MessageHandler<O>
    where
        T: 'static,
        F: FnOnce(T, AnswerSender) -> O,
    {
        match self.try_into_question::<T>() {
            Ok((arg, sender)) => {
                let val = f(arg, sender);
                MessageHandler::matched(val)
            }
            Err(this) => this,
        }
    }

    /// Calls a fallback function if the message has still not matched yet.
    ///
    /// This consumes the [`MessageHandler`], so that no matching can be
    /// performed anymore.
    pub fn on_fallback<F>(self, f: F) -> O
    where
        F: FnOnce(&dyn Any, RefAddr) -> O,
    {
        self.state
            .output_or_else(|SignedMessage { msg, sign }| f(msg.as_ref(), sign))
    }

    /// Calls a function if the incoming message is a broadcast and has a
    /// specific type.
    pub fn on_broadcast<T, F>(self, f: F) -> MessageHandler<O>
    where
        T: 'static + Send + Sync,
        F: FnOnce(&T, RefAddr) -> O,
    {
        match self.try_into_broadcast::<T>() {
            Ok((arg, addr)) => {
                let val = f(arg.as_ref(), addr);
                MessageHandler::matched(val)
            }
            Err(this) => this,
        }
    }

    /// Calls a function if the incoming message can't be replied to and has a
    /// specific type.
    pub fn on_tell<T, F>(self, f: F) -> MessageHandler<O>
    where
        T: Debug + 'static,
        F: FnOnce(T, RefAddr) -> O,
    {
        match self.try_into_tell::<T>() {
            Ok((msg, addr)) => {
                let val = f(msg, addr);
                MessageHandler::matched(val)
            }
            Err(this) => this,
        }
    }

    fn matched(output: O) -> MessageHandler<O> {
        let state = MessageHandlerState::Matched(output);
        MessageHandler { state }
    }

    fn try_into_question<T: 'static>(self) -> Result<(T, AnswerSender), MessageHandler<O>> {
        debug!("try_into_question with type {}", std::any::type_name::<T>());
        match self.state.take_message() {
            Ok(SignedMessage {
                msg:
                    Msg(MsgInner::Ask {
                        msg,
                        sender: Some(sender),
                    }),
                ..
            }) if msg.is::<T>() => {
                let msg: Box<dyn Any> = msg;
                Ok((*msg.downcast::<T>().unwrap(), sender))
            }

            Ok(anything) => Err(MessageHandler::new(anything)),
            Err(output) => Err(MessageHandler::matched(output)),
        }
    }

    fn try_into_broadcast<T: Send + Sync + 'static>(
        self,
    ) -> Result<(Arc<T>, RefAddr), MessageHandler<O>> {
        debug!(
            "try_into_broadcast with type {}",
            std::any::type_name::<T>()
        );
        match self.state.take_message() {
            Ok(SignedMessage {
                msg: Msg(MsgInner::Broadcast(msg)),
                sign,
            }) if msg.is::<T>() => {
                let msg: Arc<dyn Any + Send + Sync + 'static> = msg;
                Ok((msg.downcast::<T>().unwrap(), sign))
            }

            Ok(anything) => Err(MessageHandler::new(anything)),
            Err(output) => Err(MessageHandler::matched(output)),
        }
    }

    fn try_into_tell<T: Debug + 'static>(self) -> Result<(T, RefAddr), MessageHandler<O>> {
        debug!("try_into_tell with type {}", std::any::type_name::<T>());
        match self.state.take_message() {
            Ok(SignedMessage {
                msg: Msg(MsgInner::Tell(msg)),
                sign,
            }) if msg.is::<T>() => {
                let msg: Box<dyn Any> = msg;
                Ok((*msg.downcast::<T>().unwrap(), sign))
            }
            Ok(anything) => Err(MessageHandler::new(anything)),
            Err(output) => Err(MessageHandler::matched(output)),
        }
    }
}
