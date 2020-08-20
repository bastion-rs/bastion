//!
//! Module for gathering statistics about the run queues of the runtime
//!
//! Load balancer calculates sampled mean to provide average process execution amount
//! to all runtime.
//!
use crate::load_balancer;
use crate::placement;
use arrayvec::ArrayVec;
use fmt::{Debug, Formatter};
use lazy_static::*;
use once_cell::sync::Lazy;
use placement::CoreId;
use std::mem::MaybeUninit;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::RwLock;
use std::time::{Duration, Instant};
use std::{fmt, usize};
use tracing::{debug, error};

const MEAN_UPDATE_TRESHOLD: Duration = Duration::from_millis(200);

/// Stats of all the smp queues.
pub trait SmpStats {
    /// Stores the load of the given queue.
    fn store_load(&self, affinity: usize, load: usize);
    /// returns tuple of queue id and load ordered from highest load to lowest.
    fn get_sorted_load(&self) -> ArrayVec<[(usize, usize); MAX_CORE]>;
    /// mean of the all smp queue load.
    fn mean(&self) -> usize;
    /// update the smp mean.
    fn update_mean(&self);
}

static LOAD_BALANCER: Lazy<LoadBalancer> = Lazy::new(|| {
    let lb = LoadBalancer::new(placement::get_core_ids().unwrap());
    debug!("Instantiated load_balancer: {:?}", lb);
    lb
});

/// Load-balancer struct which allows us to update the mean load
pub struct LoadBalancer {
    /// The number of cores
    /// available for this program
    pub num_cores: usize,
    /// The core Ids available for this program
    /// This doesn't take affinity into account
    pub cores: Vec<CoreId>,
    mean_last_updated_at: RwLock<Instant>,
}

impl LoadBalancer {
    /// Creates a new LoadBalancer.
    /// if you're looking for `num_cores` and `cores`
    /// Have a look at `load_balancer::core_count()`
    /// and `load_balancer::get_cores()` respectively.
    pub fn new(cores: Vec<CoreId>) -> Self {
        Self {
            num_cores: cores.len(),
            cores,
            mean_last_updated_at: RwLock::new(Instant::now()),
        }
    }
}

impl Debug for LoadBalancer {
    fn fmt(&self, fmt: &mut Formatter) -> fmt::Result {
        fmt.debug_struct("LoadBalancer")
            .field("num_cores", &self.num_cores)
            .field("cores", &self.cores)
            .field("mean_last_updated_at", &self.mean_last_updated_at)
            .finish()
    }
}

impl LoadBalancer {
    /// Iterates the statistics to get the mean load across the cores
    pub fn update_load_mean(&self) {
        // Check if update should occur
        if !self.should_update() {
            return;
        }
        self.mean_last_updated_at
            .write()
            .map(|mut last_updated_at| {
                *last_updated_at = Instant::now();
            })
            .unwrap_or_else(|e| error!("couldn't update mean timestamp - {}", e));

        load_balancer::stats().update_mean();
    }

    fn should_update(&self) -> bool {
        // If we couldn't acquire a lock on the mean last_updated_at,
        // There is probably someone else updating already
        self.mean_last_updated_at
            .try_read()
            .map(|last_updated_at| last_updated_at.elapsed() > MEAN_UPDATE_TRESHOLD)
            .unwrap_or(false)
    }
}

/// Update the mean load on the singleton
pub fn update() {
    LOAD_BALANCER.update_load_mean()
}

/// Maximum number of core supported by modern computers.
const MAX_CORE: usize = 256;

///
/// Holding all statistics related to the run queue
///
/// Contains:
/// * Mean level of processes in the run queues
/// * SMP queue distributions
pub struct Stats {
    smp_load: [AtomicUsize; MAX_CORE],
    mean_level: AtomicUsize,
    updating_mean: AtomicBool,
}

impl fmt::Debug for Stats {
    fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        fmt.debug_struct("Stats")
            .field("smp_load", &&self.smp_load[..])
            .field("mean_level", &self.mean_level)
            .field("updating_mean", &self.updating_mean)
            .finish()
    }
}

impl Stats {
    /// new returns LockLessStats
    pub fn new(num_cores: usize) -> Stats {
        let smp_load: [AtomicUsize; MAX_CORE] = {
            let mut data: [MaybeUninit<AtomicUsize>; MAX_CORE] =
                unsafe { MaybeUninit::uninit().assume_init() };

            for core_data in data.iter_mut().take(num_cores) {
                unsafe {
                    std::ptr::write(core_data.as_mut_ptr(), AtomicUsize::new(0));
                }
            }
            for core_data in data.iter_mut().take(MAX_CORE).skip(num_cores) {
                unsafe {
                    std::ptr::write(core_data.as_mut_ptr(), AtomicUsize::new(usize::MAX));
                }
            }

            unsafe { std::mem::transmute::<_, [AtomicUsize; MAX_CORE]>(data) }
        };
        Stats {
            smp_load,
            mean_level: AtomicUsize::new(0),
            updating_mean: AtomicBool::new(false),
        }
    }
}

unsafe impl Sync for Stats {}
unsafe impl Send for Stats {}

impl SmpStats for Stats {
    fn store_load(&self, affinity: usize, load: usize) {
        self.smp_load[affinity].store(load, Ordering::SeqCst);
    }

    fn get_sorted_load(&self) -> ArrayVec<[(usize, usize); MAX_CORE]> {
        let mut sorted_load = ArrayVec::<[(usize, usize); MAX_CORE]>::new();

        for (core, load) in self.smp_load.iter().enumerate() {
            let load = load.load(Ordering::SeqCst);
            // load till maximum core.
            if load == usize::MAX {
                break;
            }
            // unsafe is ok here because self.smp_load.len() is MAX_CORE
            unsafe { sorted_load.push_unchecked((core, load)) };
        }
        sorted_load.sort_by(|x, y| y.1.cmp(&x.1));
        sorted_load
    }

    fn mean(&self) -> usize {
        self.mean_level.load(Ordering::Acquire)
    }

    fn update_mean(&self) {
        // Don't update if it's updating already
        if self.updating_mean.load(Ordering::Acquire) {
            return;
        }

        self.updating_mean.store(true, Ordering::Release);
        let mut sum: usize = 0;
        let num_cores = LOAD_BALANCER.num_cores;

        for item in self.smp_load.iter().take(num_cores) {
            if let Some(tmp) = sum.checked_add(item.load(Ordering::Acquire)) {
                sum = tmp;
            }
        }

        self.mean_level
            .store(sum.wrapping_div(num_cores), Ordering::Release);

        self.updating_mean.store(false, Ordering::Release);
    }
}

///
/// Static access to runtime statistics
#[inline]
pub fn stats() -> &'static Stats {
    lazy_static! {
        static ref LOCKLESS_STATS: Stats = Stats::new(*core_count());
    }
    &*LOCKLESS_STATS
}

///
/// Retrieve core count for the runtime scheduling purposes
#[inline]
pub fn core_count() -> &'static usize {
    &LOAD_BALANCER.num_cores
}

///
/// Retrieve cores for the runtime scheduling purposes
#[inline]
pub fn get_cores() -> &'static [CoreId] {
    &*LOAD_BALANCER.cores
}
