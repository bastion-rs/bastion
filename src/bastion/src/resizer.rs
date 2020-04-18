//!
//! Special kind of structs for providing dynamically resizable actors group.
//!
//! Features:
//! * Configuring limits and used strategies for resizers.
//! * Strategy based on statistics given by spawned actors.
//! * Auto-creation / deletion actors on demand.
//!
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

/// Special struct for scaling up and down actor groups in runtime.
pub struct Resizer {
    // Storage for actors statistics. The statistics struct is
    // represented as a sequence of bits stored in u64 value with
    // the big-endian endianness. Currently stores the amount of
    // active actors and an average mailbox size for the actor group.
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

/// Special wrapper for Arc<AtomicU64> type, that actually
/// represented as struct for storing statistical information
/// about the certain actor group.
///
/// This structure helps to provide an elegant way for handling
/// data as a sequence of bits/bytes and improve readability
/// in many places, where it supposed to be used.
///
/// The binary representation this struct for Arc<AtomicU64> can
/// be represented in the following way:
///
/// |                        AtomicU64                        |  
/// |<----------------------- 64 bits ----------------------->|
/// |                                                         |                             
/// |                    ActorGroupStats                      |
/// |<------- 32 bits ---------->|<------- 32 bits ---------->|
/// |       actors_count         |    average_mailbox_size    |
///
pub(crate) struct ActorGroupStats {
    actors_count: u32,
    average_mailbox_size: u32,
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
    /// Scaling up based on how many actors are busy and
    /// have accumulating messages in a mailbox. If actor
    /// is busy and has more message than the limit, then
    /// needs to consider re-scaling.
    ActorsUnderPressure,
    /// Scaling up based on the size of the actor's mailbox.
    MailboxSizeThreshold(u32),
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
            upscale_rate: 0.1,
            downscale_threshold: 0.3,
            downscale_rate: 0.1,
        }
    }
}

impl ActorGroupStats {
    fn actors_count_mask() -> u64 {
        0xFFFFFFFF_00000000
    }

    fn average_mailbox_size_mask() -> u64 {
        0x00000000_FFFFFFFF
    }

    /// Extract statistics from Arc<AtomicU64>
    pub(crate) fn load(storage: Arc<AtomicU64>) -> Self {
        let actors_count_mask = ActorGroupStats::actors_count_mask();
        let average_mailbox_size_mask = ActorGroupStats::average_mailbox_size_mask();

        let value = storage.load(Ordering::Relaxed);

        ActorGroupStats {
            actors_count: ((value & actors_count_mask) >> 32) as u32,
            average_mailbox_size: (value & average_mailbox_size_mask) as u32,
        }
    }

    /// Write the changes in Arc<AtomicU64>
    pub(crate) fn store(&self, storage: Arc<AtomicU64>) {
        let actors_count_mask = ActorGroupStats::actors_count_mask();
        let average_mailbox_size_mask = ActorGroupStats::average_mailbox_size_mask();

        let mut value: u64 = 0;
        value |= ((self.average_mailbox_size() as u64) & actors_count_mask);
        value |= ((self.actors_count() as u64) & actors_count_mask) << 32;

        storage.store(value, Ordering::Relaxed);
    }

    /// Returns an amount of active actors in the current group.
    pub(crate) fn actors_count(&self) -> u32 {
        self.actors_count
    }

    /// Updates the actors count value in the structure.
    pub(crate) fn update_actors_count(&mut self, value: u32) {
        self.actors_count = value
    }

    /// Returns current average mailbox size for the actors group.
    pub(crate) fn average_mailbox_size(&self) -> u32 {
        self.average_mailbox_size
    }

    /// Updates the average mailbox size in the structure.
    pub(crate) fn update_average_mailbox_size(&mut self, value: u32) {
        self.average_mailbox_size = (self.average_mailbox_size + value) / 2
    }
}

#[cfg(test)]
mod tests {}