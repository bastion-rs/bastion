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
    ($($keys:ident: $vals:expr,)*) => {
        children!(@sort,
            1,
            $crate::Callbacks::default(),
            |_| {},
            ,
            $($keys: $vals,)*
        )
    };

    (@sort,
     $_:expr, $cbs:expr, $action:expr, $($sp:expr)?,
     redundancy: $red:expr,
     $($keys:ident: $vals:expr,)*) => {
        children!(@sort,
            $red,
            $cbs,
            $action,
            $($sp)?,
            $($keys: $vals,)*
        )
    };

    (@sort,
     $red:expr, $_:expr, $action:expr, $($sp:expr)?,
     callbacks: $cbs:expr,
     $($keys:ident: $vals:expr,)*) => {
        children!(@sort,
            $red,
            $cbs,
            $action,
            $($sp)?,
            $($keys: $vals,)*
        )
    };

    (@sort,
     $red:expr, $cbs:expr, $action:expr, $($_:expr)?,
     supervisor: $sp:ident,
     $($keys:ident: $vals:expr,)*) => {
        children!(@sort,
            $red,
            $cbs,
            $action,
            $($sp)?,
            $($keys: $vals,)*
        )
    };

    (@sort,
     $red:expr, $cbs:expr, $action:expr, $($sp:expr)?,
     action: $_:expr,
     $($keys:ident: $vals:expr,)*) => {
        children!(@sort,
            $red,
            $cbs,
            $action,
            $($sp)?,
            $($keys: $vals,)*
        )
    };

    (@sort, $red:expr, $cbs:expr, $action:expr, ,) => {
        $crate::Bastion::children(|ch| {
            ch
                .with_callbacks($cbs)
                .with_redundancy($red)
                .with_exec(|ctx: $crate::context::BastionContext| {
                    async move {
                        let ctx = ctx;
                        loop {
                            let msg = ctx.recv().await?;
                            $action(msg);
                        }
                    }
                })
        });
    };
    (@sort, $red:expr, $cbs:expr, $action:expr, $sp:expr,) => {
        $sp.children(|ch| {
            ch
                .with_callbacks($cbs)
                .with_redundancy($red)
                .with_exec(|ctx: $crate::context::BastionContext| {
                    async move {
                        let ctx = ctx;
                        loop {
                            let msg = ctx.recv().await?;
                            $action(msg);
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

/// This macro creates a new supervisor with the given strategy and the given callbacks.
/// Children can be specified by using the `children` / `child` macro.
/// You can provide as many children groups as you want. Supervised supervisors are currently not
/// supported.
///
/// # Example
/// ```
/// # use bastion::prelude::*;
/// # fn main() {
/// let sp = supervisor! {
///     callbacks: Callbacks::default(),
///     strategy: SupervisionStrategy::OneForAll,
/// };
/// # }
/// ```
#[macro_export]
macro_rules! supervisor {
    ($($keys:ident: $vals:expr,)*) => {
        supervisor!(@sort,
            $crate::supervisor::SupervisionStrategy::OneForAll,
            $crate::Callbacks::default(),
            $($keys: $vals,)*
        )
    };

    (@sort,
    $strat:expr, $_:expr,
    callbacks: $cbs:expr,
    $($keys:ident: $vals:expr,)*) => {
        supervisor!(@sort,
            $strat,
            $cbs,
            $($keys: $vals),*
        )
    };

    (@sort,
    $_:expr, $cbs:expr,
    strategy: $strat:expr,
    $($keys:ident: $vals:expr,)*) => {
        supervisor!(@sort,
            $strat,
            $cbs,
            $($keys: $vals),*
        )
    };

    (@sort, $strat:expr, $cbs:expr,) => {
        $crate::Bastion::supervisor(|sp| {
            sp
                .with_callbacks($cbs)
                .with_strategy($strat)
        });
    };
}
