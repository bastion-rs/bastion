//!
//! Dynamic dispatch oriented messaging system
//!
//! This system allows:
//! * Generic communication between mailboxes.
//! * All message communication relies on at-most-once delivery guarantee.
//! * Messages are not guaranteed to be ordered, all message's order is causal.
//!
use crate::children::Children;
use crate::context::BastionId;
use crate::envelope::{RefAddr, SignedMessage};
use crate::supervisor::{SupervisionStrategy, Supervisor};
use futures::channel::oneshot::{self, Receiver};
use std::any::{type_name, Any};
use std::fmt::Debug;
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use std::task::{Context, Poll};

/// A trait that any message sent needs to implement (it is
/// already automatically implemented but forces message to
/// implement the following traits: [`Any`], [`Send`],
/// [`Sync`] and [`Debug`]).
///
/// [`Any`]: https://doc.rust-lang.org/std/any/trait.Any.html
/// [`Send`]: https://doc.rust-lang.org/std/marker/trait.Send.html
/// [`Sync`]: https://doc.rust-lang.org/std/marker/trait.Sync.html
/// [`Debug`]: https://doc.rust-lang.org/std/fmt/trait.Debug.html
pub trait Message: Any + Send + Sync + Debug {}
impl<T> Message for T where T: Any + Send + Sync + Debug {}

#[derive(Debug)]
#[doc(hidden)]
pub struct AnswerSender(oneshot::Sender<SignedMessage>);

#[derive(Debug)]
/// A [`Future`] returned when successfully "asking" a
/// message using [`ChildRef::ask`] and which resolves to
/// a `Result<Msg, ()>` where the [`Msg`] is the message
/// answered by the child (see the [`msg!`] macro for more
/// information).
///
/// # Example
///
/// ```rust
/// # use bastion::prelude::*;
/// #
/// # fn main() {
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
/// [`Future`]: https://doc.rust-lang.org/std/future/trait.Future.html
/// [`ChildRef::ask`]: ../children/struct.ChildRef.html#method.ask
/// [`Msg`]: message/struct.Msg.html
/// [`msg!`]: macro.msg.html
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
/// # fn main() {
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
/// [`BastionContext::recv`]: context/struct.BastionContext.html#method.recv
/// [`BastionContext::try_recv`]: context/struct.BastionContext.html#method.try_recv
/// [`msg!`]: macro.msg.html
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
    Deploy(Deployment),
    Prune { id: BastionId },
    SuperviseWith(SupervisionStrategy),
    Message(Msg),
    Stopped { id: BastionId },
    Faulted { id: BastionId },
}

#[derive(Debug)]
pub(crate) enum Deployment {
    Supervisor(Supervisor),
    Children(Children),
}

impl AnswerSender {
    // FIXME: we can't let manipulating Signature in a public API
    // but now it's being called only by a macro so we are trusting it
    #[doc(hidden)]
    pub fn send<M: Message>(self, msg: M, sign: RefAddr) -> Result<(), M> {
        debug!("{:?}: Sending answer: {:?}", self, msg);
        let msg = Msg::tell(msg);
        trace!("{:?}: Sending message: {:?}", self, msg);
        self.0
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

    pub(crate) fn ask<M: Message>(msg: M) -> (Self, Answer) {
        let msg = Box::new(msg);
        let (sender, recver) = oneshot::channel();
        let sender = AnswerSender(sender);
        let answer = Answer(recver);

        let sender = Some(sender);
        let inner = MsgInner::Ask { msg, sender };

        (Msg(inner), answer)
    }

    #[doc(hidden)]
    pub fn is_broadcast(&self) -> bool {
        if let MsgInner::Broadcast(_) = self.0 {
            true
        } else {
            false
        }
    }

    #[doc(hidden)]
    pub fn is_tell(&self) -> bool {
        if let MsgInner::Tell(_) = self.0 {
            true
        } else {
            false
        }
    }

    #[doc(hidden)]
    pub fn is_ask(&self) -> bool {
        if let MsgInner::Ask { .. } = self.0 {
            true
        } else {
            false
        }
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

        BastionMessage::Deploy(deployment)
    }

    pub(crate) fn deploy_children(children: Children) -> Self {
        let deployment = Deployment::Children(children);

        BastionMessage::Deploy(deployment)
    }

    pub(crate) fn prune(id: BastionId) -> Self {
        BastionMessage::Prune { id }
    }

    pub(crate) fn supervise_with(strategy: SupervisionStrategy) -> Self {
        BastionMessage::SuperviseWith(strategy)
    }

    pub(crate) fn broadcast<M: Message>(msg: M) -> Self {
        let msg = Msg::broadcast(msg);
        BastionMessage::Message(msg)
    }

    pub(crate) fn tell<M: Message>(msg: M) -> Self {
        let msg = Msg::tell(msg);
        BastionMessage::Message(msg)
    }

    pub(crate) fn ask<M: Message>(msg: M) -> (Self, Answer) {
        let (msg, answer) = Msg::ask(msg);
        (BastionMessage::Message(msg), answer)
    }

    pub(crate) fn stopped(id: BastionId) -> Self {
        BastionMessage::Stopped { id }
    }

    pub(crate) fn faulted(id: BastionId) -> Self {
        BastionMessage::Faulted { id }
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
            BastionMessage::Message(msg) => BastionMessage::Message(msg.try_clone()?),
            BastionMessage::Stopped { id } => BastionMessage::stopped(id.clone()),
            BastionMessage::Faulted { id } => BastionMessage::faulted(id.clone()),
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
        Pin::new(&mut self.get_mut().0).poll(ctx).map_err(|e| {
            // FIXME: revert logging
            error!("Failed to get answer: {:?}", e);
        })
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
/// # fn main() {
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
/// [`Msg`]: children/struct.Msg.html
/// [`BastionContext::recv`]: context/struct.BastionContext.html#method.recv
/// [`BastionContext::try_recv`]: context/struct.BastionContext.html#method.try_recv
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
                        sender.send($answer, sign)
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
