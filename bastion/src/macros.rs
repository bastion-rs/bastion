#[macro_export]
/// Matches a [`Msg`] (as returned by [`BastionContext::recv`]
/// or [`BastionContext::try_recv`]) with different types.
///
/// Each case is defined as an optional `ref` followed by a
/// variable name, a colon, a type, an arrow and the code that
/// will be executed if the message is of the specified type.
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
///             message! { msg,
///                 // We match `&'static str`s sent directly to this child...
///                 msg: &'static str => {
///                     assert_eq!(msg, TELL_MSG);
///                     // Handle the message...
///                 },
///                 // We match broadcasted `&'static str`s...
///                 ref msg: &'static str => {
///                     // Note that `msg` will actually be a `&&'static str`.
///                     assert_eq!(msg, &BCAST_MSG);
///                     // Handle the message...
///                 },
///                 // We are only broadcasting a `&'static str` in this example,
///                 // so we know that this won't happen...
///                 _: _ => (),
///             }
///         }
///
///         Ok(())
///     }.into(),
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
macro_rules! message {
    ($msg:expr, $($tokens:tt)+) => {
        { message!(@internal $msg, (), (), $($tokens)+); }
    };

    (@internal
        $msg:expr,
        ($($svar:ident, $sty:ty, $shandle:expr,)*),
        ($($ovar:ident, $oty:ty, $ohandle:expr,)*),
        ref $var:ident: $ty:ty => $handle:expr,
        $($rest:tt)+
    ) => {
        message!(@internal $msg,
            ($($svar, $sty, $shandle,)* $var, $ty, $handle,),
            ($($ovar, $oty, $ohandle,)*),
            $($rest)+
        )
    };

    (@internal
        $msg:expr,
        ($($svar:ident, $sty:ty, $shandle:expr,)*),
        ($($ovar:ident, $oty:ty, $ohandle:expr,)*),
        $var:ident: $ty:ty => $handle:expr,
        $($rest:tt)+
    ) => {
        message!(@internal $msg,
            ($($svar, $sty, $shandle,)*),
            ($($ovar, $oty, $ohandle,)* $var, $ty, $handle,),
            $($rest)+
        );
    };

    (@internal
        $msg:expr,
        ($($svar:ident, $sty:ty, $shandle:expr,)*),
        ($($ovar:ident, $oty:ty, $ohandle:expr,)*),
        _: _ => $handle:expr,
    ) => {
        message!(@internal $msg,
            ($($svar, $sty, $shandle,)*),
            ($($ovar, $oty, $ohandle,)*),
            msg: _ => $handle,
        );
    };

    (@internal
        $msg:expr,
        ($($svar:ident, $sty:ty, $shandle:expr,)*),
        ($($ovar:ident, $oty:ty, $ohandle:expr,)*),
        $var:ident: _ => $handle:expr,
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
