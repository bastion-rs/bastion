//!
//! Platform configuration for Bastion runtime
//!
use log::LevelFilter;

///
/// Configuration for platform and runtime
#[derive(Clone, Copy)]
pub struct BastionConfig {
    /// Log verbosity of runtime defined by this
    pub log_level: LevelFilter,

    /// Configure the runtime as it is running under tests.
    pub in_test: bool,
}
