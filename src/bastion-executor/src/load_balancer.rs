//!
//! Module for gathering statistics about the run queues of the runtime
//!
//! Load balancer calculates sampled mean to provide average process execution amount
//! to all runtime.
//!
use crate::load_balancer;
use crate::placement;
use arrayvec::ArrayVec;
use lazy_static::*;
use lever::sync::prelude::TTas;
use std::mem::MaybeUninit;
use std::sync::{
    atomic::{AtomicBool, AtomicUsize, Ordering},
    Arc,
};
use std::thread;
use std::time::Duration;
use std::{collections::VecDeque, fmt, usize};
use thread::Thread;

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

/// Load-balancer struct which allows us to park and unpark threads.
/// It also allows us to update the mean load
pub struct LoadBalancer {
    parked_threads: TTas<VecDeque<thread::Thread>>,
    should_update: Arc<AtomicBool>,
}

impl Default for LoadBalancer {
    fn default() -> Self {
        Self {
            parked_threads: TTas::new(VecDeque::with_capacity(
                placement::get_core_ids().unwrap().len(),
            )),
            should_update: Arc::new(AtomicBool::new(true)),
        }
    }
}

impl LoadBalancer {
    ///
    /// AMQL sampling thread for run queue load balancing.
    pub fn amql_generation() {
        thread::Builder::new()
            .name("bastion-load-balancer-thread".to_string())
            .spawn(move || {
                loop {
                    dbg!("busy looping");
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

    pub fn update_load_mean(&self) {
        // Check if update should occur
        self.should_update
            .compare_and_swap(true, false, Ordering::SeqCst);

        let cloned = Arc::clone(&self.should_update);
        thread::Builder::new()
            .name("bastion-load-balancer-thread".to_string())
            .spawn(move || {
                load_balancer::stats().update_mean();
                (*cloned).store(true, Ordering::SeqCst);
            });
    }

    pub fn park_thread(&self, thread: Thread) {
        self.parked_threads.try_lock().map(|parked_threads| {
            parked_threads.push_back(thread);
            std::thread::park();
        });
    }

    pub fn unpark_thread(&self) {
        self.parked_threads.try_lock().map(|parked_threads| {
            parked_threads.pop_front().map(|thread| thread.unpark());
        });
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

            for i in 0..num_cores {
                unsafe {
                    std::ptr::write(data[i].as_mut_ptr(), AtomicUsize::new(0));
                }
            }
            for i in num_cores..MAX_CORE {
                unsafe {
                    std::ptr::write(data[i].as_mut_ptr(), AtomicUsize::new(usize::MAX));
                }
            }

            unsafe { std::mem::transmute::<_, [AtomicUsize; MAX_CORE]>(data) }
        };
        Stats {
            smp_load,
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
        self.mean_level.load(Ordering::SeqCst)
    }

    fn update_mean(&self) {
        let mut sum: usize = 0;
        let num_cores = placement::get_core_ids().unwrap().len();

        for item in self.smp_load.iter().take(num_cores) {
            if let Some(tmp) = sum.checked_add(item.load(Ordering::SeqCst)) {
                sum = tmp;
            }
        }

        self.mean_level
            .store(sum.wrapping_div(num_cores), Ordering::SeqCst);
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
