use crate::children::Children;
use crate::context::BastionId;
use crate::supervisor::{SupervisionStrategy, Supervisor};
use std::any::Any;
use std::fmt::Debug;
use std::sync::Arc;

pub trait Message: Any + Send + Sync + Debug {}
impl<T> Message for T where T: Any + Send + Sync + Debug {}

#[derive(Debug)]
pub struct Msg(MsgInner);

#[derive(Debug)]
enum MsgInner {
    Shared(Arc<dyn Any + Send + Sync + 'static>),
    Owned(Box<dyn Any + Send + Sync + 'static>),
}

#[derive(Debug)]
pub(crate) enum BastionMessage {
    Start,
    Stop,
    Kill,
    Deploy(Deployment),
    Prune { id: BastionId },
    SuperviseWith(SupervisionStrategy),
    Tell(Msg),
    Stopped { id: BastionId },
    Faulted { id: BastionId },
}

#[derive(Debug)]
pub(crate) enum Deployment {
    Supervisor(Supervisor),
    Children(Children),
}

impl Msg {
    pub(crate) fn shared<M: Message>(msg: M) -> Self {
        let inner = MsgInner::Shared(Arc::new(msg));
        Msg(inner)
    }

    pub(crate) fn owned<M: Message>(msg: M) -> Self {
        let inner = MsgInner::Owned(Box::new(msg));
        Msg(inner)
    }

    pub fn is_broadcast(&self) -> bool {
        if let MsgInner::Shared(_) = self.0 {
            true
        } else {
            false
        }
    }

    pub fn downcast<M: Message>(self) -> Result<M, Self> {
        if let MsgInner::Owned(msg) = self.0 {
            if msg.is::<M>() {
                let msg: Box<dyn Any + 'static> = msg;
                Ok(*msg.downcast().unwrap())
            } else {
                let inner = MsgInner::Owned(msg);
                Err(Msg(inner))
            }
        } else {
            Err(self)
        }
    }

    pub fn downcast_ref<M: Message>(&self) -> Option<Arc<M>> {
        if let MsgInner::Shared(msg) = &self.0 {
            if msg.is::<M>() {
                return Some(msg.clone().downcast::<M>().unwrap());
            }
        }

        None
    }

    pub(crate) fn try_clone(&self) -> Option<Self> {
        if let MsgInner::Shared(msg) = &self.0 {
            let inner = MsgInner::Shared(msg.clone());
            Some(Msg(inner))
        } else {
            None
        }
    }

    pub(crate) fn try_unwrap<M: Message>(self) -> Result<M, Self> {
        if let MsgInner::Shared(msg) = self.0 {
            match msg.downcast() {
                Ok(msg) => match Arc::try_unwrap(msg) {
                    Ok(msg) => Ok(msg),
                    Err(msg) => {
                        let inner = MsgInner::Shared(msg);
                        Err(Msg(inner))
                    }
                },
                Err(msg) => {
                    let inner = MsgInner::Shared(msg);
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
        let msg = Msg::shared(msg);
        BastionMessage::Tell(msg)
    }

    pub(crate) fn tell<M: Message>(msg: M) -> Self {
        let msg = Msg::owned(msg);
        BastionMessage::Tell(msg)
    }

    pub(crate) fn stopped(id: BastionId) -> Self {
        BastionMessage::Stopped { id }
    }

    pub(crate) fn faulted(id: BastionId) -> Self {
        BastionMessage::Faulted { id }
    }

    pub(crate) fn try_clone(&self) -> Option<Self> {
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
            BastionMessage::Tell(msg) => BastionMessage::Tell(msg.try_clone()?),
            BastionMessage::Stopped { id } => BastionMessage::stopped(id.clone()),
            BastionMessage::Faulted { id } => BastionMessage::faulted(id.clone()),
        };

        Some(clone)
    }

    pub(crate) fn into_msg<M: Message>(self) -> Option<M> {
        if let BastionMessage::Tell(msg) = self {
            msg.try_unwrap().ok()
        } else {
            None
        }
    }
}

#[macro_export]
/// Matches a [`Msg`] (as returned by [`BastionContext::recv`]
/// or [`BastionContext::try_recv`]) with different types.
///
/// Each case is defined as an optional `ref` followed by a
/// variable name, a colon, a type, an arrow, the code that
/// will be executed if the message is of the specified type,
/// and finally a semicolon.
///
/// Specifying a `ref` will make the case only match if the
/// message was broadcasted while not indicating it will
/// only match if it was sent directly to the child.
///
/// Only a default case is required, which is defined as any
/// other case but with its type set as `_`.
///
/// # Example
///
/// ```
/// # use bastion::prelude::*;
/// #
/// # fn main() {
///     # Bastion::init();
/// // The message that will be broadcasted...
/// const BCAST_MSG: &'static str = "A message containing data (broadcast).";
/// // The message that will be sent to the child directly...
/// const TELL_MSG: &'static str = "A message containing data (tell).";
///
/// Bastion::children(|ctx: BastionContext|
///     async move {
///         # ctx.current().send_msg(TELL_MSG).unwrap();
///         loop {
///
///             let msg = ctx.recv().await?;
///             msg! { msg,
///                 // We match `&'static str`s sent directly to this child...
///                 msg: &'static str => {
///                     assert_eq!(msg, TELL_MSG);
///                     // Handle the message...
///                 };
///                 // We match broadcasted `&'static str`s...
///                 ref msg: &'static str => {
///                     // Note that `msg` will actually be a `&&'static str`.
///                     assert_eq!(msg, &BCAST_MSG);
///                     // Handle the message...
///                 };
///                 // We are only broadcasting a `&'static str` in this example,
///                 // so we know that this won't happen...
///                 _: _ => ();
///             }
///         }
///
///         Ok(())
///     },
///     1,
/// ).expect("Couldn't start the children group.");
///     #
///     # Bastion::start();
///     # Bastion::broadcast(BCAST_MSG).unwrap();
///     # Bastion::stop();
///     # Bastion::block_until_stopped();
/// # }
/// ```
///
/// [`Msg`]: children/struct.Msg.html
/// [`BastionContext::recv`]: struct.BastionContext.html#method.recv
/// [`BastionContext::try_recv`]: struct.BastionContext.html#method.try_recv
macro_rules! msg {
    ($msg:expr, $($tokens:tt)+) => {
        { msg!(@internal $msg, (), (), $($tokens)+); }
    };

    (@internal
        $msg:expr,
        ($($svar:ident, $sty:ty, $shandle:expr,)*),
        ($($ovar:ident, $oty:ty, $ohandle:expr,)*),
        ref $var:ident: $ty:ty => $handle:expr;
        $($rest:tt)+
    ) => {
        msg!(@internal $msg,
            ($($svar, $sty, $shandle,)* $var, $ty, $handle,),
            ($($ovar, $oty, $ohandle,)*),
            $($rest)+
        )
    };

    (@internal
        $msg:expr,
        ($($svar:ident, $sty:ty, $shandle:expr,)*),
        ($($ovar:ident, $oty:ty, $ohandle:expr,)*),
        $var:ident: $ty:ty => $handle:expr;
        $($rest:tt)+
    ) => {
        msg!(@internal $msg,
            ($($svar, $sty, $shandle,)*),
            ($($ovar, $oty, $ohandle,)* $var, $ty, $handle,),
            $($rest)+
        )
    };

    (@internal
        $msg:expr,
        ($($svar:ident, $sty:ty, $shandle:expr,)*),
        ($($ovar:ident, $oty:ty, $ohandle:expr,)*),
        _: _ => $handle:expr;
    ) => {
        msg!(@internal $msg,
            ($($svar, $sty, $shandle,)*),
            ($($ovar, $oty, $ohandle,)*),
            msg: _ => $handle;
        )
    };

    (@internal
        $msg:expr,
        ($($svar:ident, $sty:ty, $shandle:expr,)*),
        ($($ovar:ident, $oty:ty, $ohandle:expr,)*),
        $var:ident: _ => $handle:expr;
    ) => {
        let mut $var = $msg;
        if $var.is_broadcast() {
            if false {}
            $(
                else if let Some($svar) = $var.downcast_ref::<$sty>() {
                    let $svar = &*$svar;
                    $shandle
                }
            )*
            else {
                $handle
            }
        } else {
            loop {
                $(
                    match $var.downcast::<$oty>() {
                        Ok($ovar) => {
                            $ohandle
                            break;
                        }
                        Err(msg_) => $var = msg_,
                    }
                )*

                $handle
            }
        }
    };
}
