//!
//! NUMA-aware locality enabled allocator with optional fallback.
//!
//! Currently this API marked as `unstable` and can only be used with `unstable` feature.
//!
//! This allocator checks for NUMA-aware locality, and if it suitable it can start
//! this allocator with local allocation policy [MPOL_LOCAL].
//! In other cases otherwise it tries to use jemalloc.
//!
//! This allocator is an allocator called [Numanji].
//!
//! [Numanji]: https://docs.rs/numanji
//! [MPOL_LOCAL]: http://man7.org/linux/man-pages/man2/set_mempolicy.2.html
//!
unstable_api! {
    // Allocation selector import
    use numanji::*;

    // Drive selection of allocator here
    autoselect!();
}
