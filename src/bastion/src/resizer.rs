//!
//! Special kind of structs for providing dynamically resizable actors group.
//!
//! Features:
//! * Configuring limits and used strategies for resizers.
//! * Strategy based on statistics given by spawned actors.
//! * Auto-creation / deletion actors on demand.
//!
use std::sync::atomic::AtomicU64;
use std::sync::Arc;

/// Special struct for scaling up and down actor groups in runtime.
pub struct Resizer {
    // Storage for actors statistics.
    stats: Arc<AtomicU64>,
    // The minimal amount of actors that must be active.
    lower_bound: u64,
    // The maximal amount of actors acceptable for usage.
    upper_bound: UpperBoundLimit,
    // The usage strategy for scaling up actors in runtime.
    upscale_strategy: UpscaleStrategy,
    // Determines how much more actors needs to be created (in percentages
    // relatively to the active actors).
    upscale_rate: f64,
    // The minimum percentage of busy actors before scaling down.
    downscale_threshold: f64,
    // Determines how much actors needs to be removed (in percentages
    // relatively to the active actors).
    downscale_rate: f64,
}

/// An enumeration that describe acceptable upper boundaries
/// for the spawned actors in runtime.
pub enum UpperBoundLimit {
    /// Sets the limit for maximum available actors to use.   
    Limit(u64),
    /// No limits for spawning new actors.
    Unlimited,
}

/// Determines the strategy for scaling up in runtime.
pub enum UpscaleStrategy {
    /// Scaling up based on how much of actors are busy.
    ActorsAreBusy,
    /// Scaling up based on how much actors are busy and have
    /// accumulating messages in a mailbox.
    ActorsUnderPressure,
    /// Scaling up based on the size of the actor's mailbox.
    MailboxSizeThreshold(u64),
}

impl Resizer {
    /// Overrides the minimal amount of actors available to use.
    pub fn with_lower_bound(mut self, lower_bound: u64) -> Self {
        self.lower_bound = lower_bound;
        self
    }

    /// Overrides the maximum amount of actors available to use.
    pub fn with_upper_bound(mut self, upper_bound: UpperBoundLimit) -> Self {
        self.upper_bound = upper_bound;
        self
    }

    /// Overrides the upscale strategy to use.
    pub fn with_upscale_strategy(mut self, upscale_strategy: UpscaleStrategy) -> Self {
        self.upscale_strategy = upscale_strategy;
        self
    }

    /// Overrides the upscale rate for actors.
    pub fn with_upscale_rate(mut self, upscale_rate: f64) -> Self {
        self.upscale_rate = upscale_rate;
        self
    }

    /// Overrides the downscale threshold for resizer.
    pub fn with_downscale_threshold(mut self, downscale_threshold: f64) -> Self {
        self.downscale_threshold = downscale_threshold;
        self
    }

    /// Overrides the downscale rate for actors.
    pub fn with_downscale_rate(mut self, downscale_rate: f64) -> Self {
        self.downscale_rate = downscale_rate;
        self
    }

    /// Returns an atomic reference to data with actor statistics.
    pub(crate) fn stats(&self) -> Arc<AtomicU64> {
        self.stats.clone()
    }

    // TODO: Add implementation
    pub(crate) fn get_actors_count(self) -> u64 {
        0
    }

    // TODO: Add implementation
    pub(crate) fn get_average_time_execution(self) -> u64 {
        0
    }

    // TODO: Add implementation
    pub(crate) fn get_average_mailbox_size(self) -> u64 {
        0
    }

    // TODO: Add implementation
    pub(crate) fn get_(self) -> u64 {
        0
    }

    // TODO: Add implementation
    pub(crate) fn get_median_mailbox_size(self) -> u64 {
        0
    }

    /// Applies checks and does scaling up/down depends on stats.
    /// TODO: Measure the execution time?
    pub(crate) async fn scale(&self) {}
}

impl Default for Resizer {
    fn default() -> Self {
        Resizer {
            stats: Arc::new(AtomicU64::new(0)),
            lower_bound: 1,
            upper_bound: UpperBoundLimit::Limit(10),
            upscale_strategy: UpscaleStrategy::ActorsAreBusy,
            upscale_rate: 0.2,
            downscale_threshold: 0.3,
            downscale_rate: 0.1,
        }
    }
}
