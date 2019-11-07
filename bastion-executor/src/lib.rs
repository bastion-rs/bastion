//!
//!
//!
//! NUMA-aware SMP based Fault-tolerant Executor
//!
//!

// Discarded lints
#![allow(clippy::if_same_then_else)]
// Force missing implementations
#![warn(missing_docs)]
#![warn(missing_debug_implementations)]

#[macro_use]
mod macros;

pub mod allocator;
pub mod blocking_pool;
pub mod distributor;
pub mod load_balancer;
pub mod placement;
pub mod pool;
pub mod run;
pub mod run_queue;
pub mod sleepers;
pub mod thread_recovery;
pub mod worker;

pub mod prelude {
    pub use crate::pool::*;
    pub use crate::run::*;
}
