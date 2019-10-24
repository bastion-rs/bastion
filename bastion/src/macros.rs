#[macro_export]
/// Matches a boxed [`Message`] (`Box<dyn Message>` as returned
/// by [`BastionContext::recv`] or [`BastionContext::try_recv`])
/// with different types.
///
/// Each case is defined as a variable name followed by a colon,
/// a type, and arrow and the code that will be executed if the
/// message is of the specified type.
///
/// Only a default case is required, which is defined in the
/// same way a `match` would define one (`_ => { ... }`).
///
/// # Example
///
/// ```
/// # use bastion::prelude::*;
/// #
/// # fn main() {
///     # Bastion::init();
/// // The message that will be sent...
/// let msg = "A message containing data.";
///
/// Bastion::children(|ctx: BastionContext|
///     async move {
///         let msg = ctx.recv().await?;
///         message! { msg,
///             msg: &'static str => {
///                 assert_eq!(msg, &"A message containing data.");
///             },
///             // We are only sending a `&'static str` in this example,
///             // so we know that this won't happen...
///             _ => unreachable!(),
///         }
///
///         Ok(())
///     }.into(),
///     1,
/// ).expect("Couldn't start the children group.");
///     #
///     # Bastion::start();
///     # Bastion::broadcast(Box::new(msg)).unwrap();
///     # Bastion::stop();
///     # Bastion::block_until_stopped();
/// # }
/// ```
///
/// [`Message`]: children/trait.Message.html
/// [`BastionContext::recv`]: struct.BastionContext.html#method.recv
/// [`BastionContext::try_recv`]: struct.BastionContext.html#method.try_recv
macro_rules! message {
    ($msg:expr,
        $($var:ident: $ty:ty => $handle:expr,)*
        _ => $fallback:expr,
    ) => {
        if false {}
        $(
            if let Some($var) = $msg.as_any().downcast_ref::<$ty>() {
                $handle
            }
        )*
        else {
            $fallback
        }
    };
}
