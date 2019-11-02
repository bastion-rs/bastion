//!
//!
//!
//! NUMA-aware SMP based Fault-tolerant Executor
//!

// Executing asm in some places
#![feature(asm)]
// Allocator features
#![feature(allocator_api)]
#![feature(extern_types)]
#![feature(core_intrinsics)]
#![feature(libstd_sys_internals)]
#![feature(thread_local)]
#![feature(const_fn)]
// Force missing implementations
#![warn(missing_docs)]
#![warn(missing_debug_implementations)]

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
