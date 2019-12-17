//! This module contains some useful macros to simply
//! create supervisors and children groups.
#![allow(unused)]

/// This macro creates a new children group with the given amount of workers
/// callbacks, and a closure which should be executed when a message is received.
///
/// # Example
///
/// ```
/// # use bastion::prelude::*;
/// # fn main() {
/// // This creates a new children group with 100 workers
/// // and will call the closure every time a message is received.
/// let children_without_callbacks = children! { 100,
///     ctx, msg => {
///         // use ctx, and msg here
///     }
/// };
///
/// let callbacks = Callbacks::new()
///     .with_before_start(|| println!("Children group started."))
///     .with_after_stop(|| println!("Children group stopped."));
/// // This creates the same children group as above, but with your own callbacks
/// let children_with_callbacks = children! { 100, callbacks,
///     ctx, msg => {
///         // use ctx, and msg here
///     }
/// };
/// # }
/// ```
#[macro_export]
macro_rules! children {
    ($count:expr, $ctx:ident, $msg:ident => $code:block) => {
        children!($count, $crate::Callbacks::default(), $ctx, $msg => $code)
    };

    ($count:expr, $callbacks:expr, $ctx:ident, $msg:ident => $code:block) => {
        $crate::Bastion::children(|children: $crate::children::Children| {
            children
                .with_redundancy($count)
                .with_callbacks($callbacks)
                .with_exec(|ctx: $crate::context::BastionContext| {
                    async move {
                        let $ctx = ctx;
                        loop {
                            let $msg = $ctx.recv().await?;
                            $code;
                        }
                    }
                })
        });
    };
}

/// This macro creates a new children group with the only one worker and the given
/// closure as action.
///
/// # Example
///
/// ```
/// # use bastion::prelude::*;
/// # fn main() {
/// // This creates a new children group with 1 worker
/// // and will call the closure every time a message is received.
/// let children_without_callbacks = child! {
///     ctx, msg => {
///         // use ctx, and msg here
///     }
/// };
///
/// let callbacks = Callbacks::new()
///     .with_before_start(|| println!("Children group started."))
///     .with_after_stop(|| println!("Children group stopped."));
/// // This creates the same children group as above, but with your own callbacks
/// let children_with_callbacks = child! { callbacks,
///     ctx, msg => {
///         // use ctx, and msg here
///     }
/// };
/// # }
/// ```
#[macro_export]
macro_rules! child {
    ($ctx:ident, $msg:ident => $code:block) => {
        child!($crate::Callbacks::default(),  $ctx, $msg => $code)
    };
    ($callbacks:expr, $ctx:ident, $msg:ident => $code:block) => {
        children!(1, $callbacks, $ctx, $msg => $code)
    };
}
