//!
//! Module for gathering statistics about the run queues of the runtime
//!
//! Load balancer calculates sampled mean to provide average process execution amount
//! to all runtime.
//!
use crate::load_balancer;
use crate::placement;
use lazy_static::*;
use std::mem::MaybeUninit;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::thread;
use std::time::Duration;
use std::{fmt, usize};

/// Stats of all the smp queues.
pub trait SmpStats {
    /// Stores the load of the given queue.
    fn store_load(&self, affinity: usize, load: usize);
    /// returns tuple of queue id and load in an sorted order.
    fn get_sorted_load(&self) -> Vec<(usize, usize)>;
    /// mean of the all smp queue load.
    fn mean(&self) -> usize;
    /// update the smp mean.
    fn update_mean(&self);
}
///
/// Load-balancer struct which is just a convenience wrapper over the statistics calculations.
#[derive(Debug)]
pub struct LoadBalancer;

impl LoadBalancer {
    ///
    /// AMQL sampling thread for run queue load balancing.
    pub fn amql_generation() {
        thread::Builder::new()
            .name("bastion-load-balancer-thread".to_string())
            .spawn(move || {
                loop {
                    load_balancer::stats().update_mean();
                    // We don't have β-reduction here… Life is unfair. Life is cruel.
                    //
                    // Try sleeping for a while to wait
                    // Should be smaller time slice than 4 times per second to not miss
                    thread::sleep(Duration::from_millis(245));
                    // Yield immediately back to os so we can advance in workers
                    thread::yield_now();
                }
            })
            .expect("load-balancer couldn't start");
    }
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
}

impl fmt::Debug for Stats {
    fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        fmt.debug_struct("Stats")
            .field("smp_load", &&self.smp_load[..])
            .field("mean_level", &self.mean_level)
            .finish()
    }
}

impl Stats {
    /// new returns LockLessStats
    pub fn new(num_cores: usize) -> Stats {
        let smp_load: [AtomicUsize; MAX_CORE] = {
            let mut data: [MaybeUninit<AtomicUsize>; MAX_CORE] =
                unsafe { MaybeUninit::uninit().assume_init() };
            let mut i = 0;
            while i < MAX_CORE {
                if i < num_cores {
                    unsafe {
                        std::ptr::write(data[i].as_mut_ptr(), AtomicUsize::new(0));
                    }
                    i = i + 1;
                    continue;
                }
                // MAX is for unused slot.
                unsafe {
                    std::ptr::write(data[i].as_mut_ptr(), AtomicUsize::new(usize::MAX));
                }
                i = i + 1;
            }
            unsafe { std::mem::transmute::<_, [AtomicUsize; MAX_CORE]>(data) }
        };
        Stats {
            smp_load: smp_load,
            mean_level: AtomicUsize::new(0),
        }
    }
}

unsafe impl Sync for Stats {}
unsafe impl Send for Stats {}

impl SmpStats for Stats {
    fn store_load(&self, affinity: usize, load: usize) {
        self.smp_load[affinity].store(load, Ordering::SeqCst);
    }

    fn get_sorted_load(&self) -> Vec<(usize, usize)> {
        let mut sorted_load = Vec::new();

        for (i, item) in self.smp_load.iter().enumerate() {
            let load = item.load(Ordering::SeqCst);
            // load till maximum core.
            if load == usize::MAX {
                break;
            }
            sorted_load.push((i, load));
        }
        sorted_load.sort_by(|x, y| y.1.cmp(&x.1));
        sorted_load
    }

    fn mean(&self) -> usize {
        self.mean_level.load(Ordering::SeqCst)
    }

    fn update_mean(&self) {
        let mut sum: usize = 0;

        for item in self.smp_load.iter() {
            let load = item.load(Ordering::SeqCst);
            if let Some(tmp) = sum.checked_add(load) {
                sum = tmp;
                continue;
            }
            break;
        }
        self.mean_level.store(
            sum.wrapping_div(placement::get_core_ids().unwrap().len()),
            Ordering::SeqCst,
        );
    }
}

///
/// Static access to runtime statistics
#[inline]
pub fn stats() -> &'static Stats {
    lazy_static! {
        static ref LOCKLESS_STATS: Stats = Stats::new(*core_retrieval());
    }
    &*LOCKLESS_STATS
}

///
/// Retrieve core count for the runtime scheduling purposes
#[inline]
pub fn core_retrieval() -> &'static usize {
    lazy_static! {
        static ref CORE_COUNT: usize = placement::get_core_ids().unwrap().len();
    }

    &*CORE_COUNT
}
