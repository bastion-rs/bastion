//! This module contains some useful macros to simply
//! create supervisors and children groups.

/// This macro creates a new children group with the given amount of worker
/// callbacks, and a closure that will be executed when a message is received.
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
/// yet supported.
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

/// Spawns a blocking task, which will run on the blocking thread pool,
/// and returns the handle.
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
/// run!(task);
/// # }
/// ```
#[macro_export]
macro_rules! blocking {
    ($($tokens:tt)*) => {
        $crate::executor::blocking(async move {
            $($tokens)*
        })
    };
}

/// This macro blocks the current thread until passed
/// future is resolved with an output (including the panic).
///
/// # Example
/// ```
/// # use bastion::prelude::*;
/// # fn main() {
/// let future1 = async move {
///     123
/// };
///
/// run! {
///     let result = future1.await;
///     assert_eq!(result, 123);
/// };
///
/// let future2 = async move {
///     10 / 2
/// };
///
/// let result = run!(future2);
/// assert_eq!(result, 5);
/// # }
/// ```
#[macro_export]
macro_rules! run {
    ($action:expr) => {
        $crate::executor::run($action)
    };

    ($($tokens:tt)*) => {
        bastion::executor::run(async move {$($tokens)*})
    };
}

/// Spawn a given future onto the executor from the global level.
///
/// # Example
/// ```
/// # use bastion::prelude::*;
/// # fn main() {
/// let handle = spawn! {
///     panic!("test");
/// };
/// run!(handle);
/// # }
/// ```
#[macro_export]
macro_rules! spawn {
    ($action:expr) => {
        $crate::executor::spawn($action)
    };

    ($($tokens:tt)*) => {
        bastion::executor::spawn(async move {$($tokens)*})
    };
}

///
/// Marker of distributed API.
#[doc(hidden)]
macro_rules! distributed_api {
    ($($block:item)*) => {
        $(
            #[cfg(feature = "distributed")]
            #[cfg_attr(feature = "docs", doc(cfg(distributed)))]
            $block
        )*
    }
}
