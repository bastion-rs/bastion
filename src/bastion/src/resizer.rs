//!
//! Special kind of structs for resizable actor groups in runtime.
//!
//! Features:
//! * Configuring limits and used strategies for resizers.
//! * Strategy based on statistics given by spawned actors.
//! * Auto-creation / deletion actors on demand.
//!
use crate::broadcast::Sender;
use crate::context::BastionId;
use fxhash::FxHashMap;
use lightproc::recoverable_handle::RecoverableHandle;
use std::cmp::min;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

#[derive(Debug)]
/// Special struct for scaling up and down actor groups in runtime.
pub struct OptimalSizeExploringResizer {
    // Storage for actors statistics. The statistics struct is
    // represented as a sequence of bits stored in u64 value with
    // the big-endian endianness. Currently stores the amount of
    // active actors and an average mailbox size for the actor group.
    stats: Arc<AtomicU64>,
    // The minimal amount of actors that must be active.
    lower_bound: u64,
    // The maximal amount of actors acceptable for usage.
    upper_bound: UpperBound,
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

#[derive(Debug)]
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

#[derive(Debug, Clone)]
/// An enumeration that describe acceptable upper boundaries
/// for the spawned actors in runtime.
pub enum UpperBound {
    /// Sets the limit for maximum available actors to use.   
    Limit(u64),
    /// No limits for spawning new actors.
    Unlimited,
}

#[derive(Debug, Clone)]
/// Determines the strategy for scaling up in runtime.
pub enum UpscaleStrategy {
    /// Scaling up based on the size of the actor's mailbox.
    MailboxSizeThreshold(u32),
}

#[derive(Debug)]
/// Determines what action needs to be applied by the caller
/// after Resizer checks.
pub(crate) enum ScalingRule {
    /// Specifies how much more actors must be instantiated.
    Upscale(u64),
    /// Defines what actors must be stopped or removed.
    Downscale(Vec<BastionId>),
    /// Special result kind that defines that no needed to scale up/down.
    DoNothing,
}

impl OptimalSizeExploringResizer {
    /// Returns an atomic reference to data with actor statistics.
    pub(crate) fn stats(&self) -> Arc<AtomicU64> {
        self.stats.clone()
    }

    /// Overrides the minimal amount of actors available to use.
    pub fn with_lower_bound(mut self, lower_bound: u64) -> Self {
        self.lower_bound = lower_bound;
        self
    }

    /// Overrides the maximum amount of actors available to use.
    pub fn with_upper_bound(mut self, upper_bound: UpperBound) -> Self {
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

    /// Applies checks and does scaling up/down depends on stats.
    pub(crate) async fn scale(
        &self,
        actors: &FxHashMap<BastionId, (Sender, RecoverableHandle<()>)>,
    ) -> ScalingRule {
        let mut stats = ActorGroupStats::load(self.stats.clone());

        // Scaling up
        match self.upscale_strategy {
            UpscaleStrategy::MailboxSizeThreshold(threshold) => {
                if stats.average_mailbox_size > threshold {
                    let desired_upscale =
                        (stats.actors_count as f64 * self.upscale_rate).round() as u64;
                    stats.average_mailbox_size = 0;
                    stats.store(self.stats.clone());
                    return self.adjustment_upscaling(actors, desired_upscale);
                }
            }
        };

        // Scaling down
        let mut actors_to_stop = Vec::new();
        for (actor_id, (_, handle)) in actors {
            let state = handle.state();

            if state.is_closed() || state.is_completed() {
                actors_to_stop.push(actor_id.clone())
            }
        }
        if !actors.is_empty() {
            let freed_actors_max = (self.downscale_rate * actors.len() as f64).round() as usize;
            let freed_actors_limit = min(actors_to_stop.len(), freed_actors_max);
            let freed_actors = actors_to_stop.drain(0..freed_actors_limit).collect();
            return ScalingRule::Downscale(freed_actors);
        }

        ScalingRule::DoNothing
    }

    fn adjustment_upscaling(
        &self,
        actors: &FxHashMap<BastionId, (Sender, RecoverableHandle<()>)>,
        desired_upscale: u64,
    ) -> ScalingRule {
        match self.upper_bound {
            UpperBound::Limit(actors_limit) => {
                let active_actors = actors.len() as u64;
                let can_be_added_actors_max = actors_limit - active_actors;
                let added_actors_limit = min(desired_upscale, can_be_added_actors_max);

                match added_actors_limit {
                    0 => ScalingRule::DoNothing,
                    _ => ScalingRule::Upscale(added_actors_limit),
                }
            }
            UpperBound::Unlimited => ScalingRule::Upscale(desired_upscale),
        }
    }
}

impl Default for OptimalSizeExploringResizer {
    fn default() -> Self {
        OptimalSizeExploringResizer {
            stats: Arc::new(AtomicU64::new(0)),
            lower_bound: 1,
            upper_bound: UpperBound::Limit(10),
            upscale_strategy: UpscaleStrategy::MailboxSizeThreshold(3),
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

    /// Extract statistics from Arc<AtomicU64>.
    pub(crate) fn load(storage: Arc<AtomicU64>) -> Self {
        let actors_count_mask = ActorGroupStats::actors_count_mask();
        let average_mailbox_size_mask = ActorGroupStats::average_mailbox_size_mask();

        let value = storage.load(Ordering::SeqCst);

        ActorGroupStats {
            actors_count: ((value & actors_count_mask) >> 32) as u32,
            average_mailbox_size: (value & average_mailbox_size_mask) as u32,
        }
    }

    /// Write the changes in Arc<AtomicU64>.
    pub(crate) fn store(&self, storage: Arc<AtomicU64>) {
        let actors_count_mask = ActorGroupStats::actors_count_mask();
        let average_mailbox_size_mask = ActorGroupStats::average_mailbox_size_mask();

        let mut value: u64 = 0;
        value |= (self.average_mailbox_size() as u64) & average_mailbox_size_mask;
        value |= ((self.actors_count() as u64) << 32) & actors_count_mask;

        storage.store(value, Ordering::SeqCst);
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
        let avg = (self.average_mailbox_size + value) as f32 / 2.0;
        self.average_mailbox_size = avg.floor() as u32;
    }
}

#[cfg(test)]
mod tests {
    use crate::resizer::{ActorGroupStats, OptimalSizeExploringResizer};

    #[test]
    fn test_resizer_stores_empty_stats_by_default() {
        let resizer = OptimalSizeExploringResizer::default();

        let stats = ActorGroupStats::load(resizer.stats);
        assert_eq!(stats.actors_count, 0);
        assert_eq!(stats.average_mailbox_size, 0);
    }

    #[test]
    fn test_resizer_returns_refreshed_stats_after_actors_count_update() {
        let resizer = OptimalSizeExploringResizer::default();
        let atomic_stats = resizer.stats();

        let mut stats = ActorGroupStats::load(atomic_stats.clone());
        stats.update_actors_count(10);
        stats.store(atomic_stats);

        let updated_stats = ActorGroupStats::load(resizer.stats);
        assert_eq!(updated_stats.actors_count, 10);
        assert_eq!(updated_stats.average_mailbox_size, 0);
    }

    #[test]
    fn test_resizer_returns_refreshed_stats_after_avg_mailbox_size_update() {
        let resizer = OptimalSizeExploringResizer::default();
        let atomic_stats = resizer.stats();

        let mut stats = ActorGroupStats::load(atomic_stats.clone());
        stats.update_average_mailbox_size(100);
        stats.store(atomic_stats);

        let updated_stats = ActorGroupStats::load(resizer.stats);
        assert_eq!(updated_stats.actors_count, 0);
        assert_eq!(updated_stats.average_mailbox_size, 50);
    }

    #[test]
    fn test_resizer_returns_refreshed_stats_after_actor_count_and_avg_mailbox_size_update() {
        let resizer = OptimalSizeExploringResizer::default();
        let atomic_stats = resizer.stats();

        let mut stats = ActorGroupStats::load(atomic_stats.clone());
        stats.update_actors_count(10);
        stats.update_average_mailbox_size(100);
        stats.store(atomic_stats);

        let updated_stats = ActorGroupStats::load(resizer.stats);
        assert_eq!(updated_stats.actors_count, 10);
        assert_eq!(updated_stats.average_mailbox_size, 50);
    }
}
