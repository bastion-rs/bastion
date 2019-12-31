//! This module contains some useful macros to simply
//! create supervisors and children groups.

/// This macro creates a new children group with the given amount of workers
/// callbacks, and a closure which should be executed when a message is received.
///
/// # Example
///
/// ```
/// # use bastion::prelude::*;
/// # fn main() {
/// let children = children! {
///     // the default redundancy is 1
///     redundancy: 100,
///     action: |msg| {
///         // do something with the message here
///     },
/// };
///
/// let sp = supervisor! {};
/// let children = children! {
///     supervisor: sp,
///     redundancy: 10,
///     action: |msg| {
///         // do something with the message here
///     },
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
     supervisor: $sp:expr,
     $($keys:ident: $vals:expr,)*) => {
        children!(@sort,
            $red,
            $cbs,
            $action,
            $sp,
            $($keys: $vals,)*
        )
    };

    (@sort,
     $red:expr, $cbs:expr, $_:expr, $($sp:expr)?,
     action: $action:expr,
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
                            ($action)(msg);
                        }
                    }
                })
        }).expect("failed to create children group");
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
                            ($action)(msg);
                        }
                    }
                })
        }).expect("failed to create children group");
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
            $($keys: $vals,)*
        )
    };

    (@sort,
    $_:expr, $cbs:expr,
    strategy: $strat:expr,
    $($keys:ident: $vals:expr,)*) => {
        supervisor!(@sort,
            $strat,
            $cbs,
            $($keys: $vals,)*
        )
    };

    (@sort, $strat:expr, $cbs:expr,) => {
        $crate::Bastion::supervisor(|sp| {
            sp
                .with_callbacks($cbs)
                .with_strategy($strat)
        }).expect("failed to create supervisor");
    };
}

/// Spawnes a blocking task, which will be run on an extra thread pool, and
/// returns the handle.
///
/// # Example
/// ```
/// # use std::{thread, time};
/// # use lightproc::proc_stack::ProcStack;
/// # use bastion::prelude::*;
/// # fn main() {
/// let task = blocking! {
///     thread::sleep(time::Duration::from_millis(3000));
/// };
/// bastion_executor::run::run(task, ProcStack::default());
/// # }
/// ```
#[macro_export]
macro_rules! blocking {
    ($($tokens:tt)*) => {
        bastion_executor::blocking::spawn_blocking(async move {
            $($tokens)*
        }, lightproc::proc_stack::ProcStack::default())
    };
}
