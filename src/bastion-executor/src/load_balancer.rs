//!
//! Module for gathering statistics about the run queues of the runtime
//!
//! Load balancer calculates sampled mean to provide average process execution amount
//! to all runtime.
//!
use crate::load_balancer;
use crate::placement;
use arrayvec::ArrayVec;
use crossbeam_queue::ArrayQueue;
use fmt::{Debug, Formatter};
use lazy_static::*;
use std::mem::MaybeUninit;
use std::sync::{
    atomic::{AtomicBool, AtomicUsize, Ordering},
    Arc,
};
use std::thread;
use std::time::Duration;
use std::{fmt, usize};
use thread::Thread;
use tracing::*;

/// The timeout we'll use when parking the last awaken thread  
pub const THREAD_PARK_TIMEOUT: Duration = Duration::from_millis(1);

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
    num_cores: usize,
    parked_threads: ArrayQueue<Thread>,
    should_update_load_mean: Arc<AtomicBool>,
    draining_parked_threads: AtomicBool,
}

impl Debug for LoadBalancer {
    fn fmt(&self, fmt: &mut Formatter) -> fmt::Result {
        fmt.debug_struct("LoadBalancer")
            .field("num_cores", &self.num_cores)
            .field(
                "should_update_load_mean",
                &self.should_update_load_mean.load(Ordering::Relaxed),
            )
            .field("num_parked_threads", &self.parked_threads.len())
            .field("draining_parked_threads", &self.draining_parked_threads)
            .finish()
    }
}

impl Default for LoadBalancer {
    fn default() -> Self {
        let num_cores = placement::get_num_cores().unwrap();
        info!(
            "Bastion-executor: Instanciating LoadBalancer with {} cores.",
            num_cores
        );
        Self {
            num_cores,
            // The last parked thread will be parked with a timeout, we won't add it here
            parked_threads: ArrayQueue::new(num_cores - 1),
            should_update_load_mean: Arc::new(AtomicBool::new(true)),
            draining_parked_threads: AtomicBool::new(false),
        }
    }
}

impl LoadBalancer {
    /// Iterates the statistics to get the mean load across the cores
    pub fn update_load_mean(&self) {
        // Check if update should occur
        self.should_update_load_mean
            .compare_and_swap(true, false, Ordering::SeqCst);

        let cloned = Arc::clone(&self.should_update_load_mean);
        thread::Builder::new()
            .name("bastion-load-balancer-thread".to_string())
            .spawn(move || {
                load_balancer::stats().update_mean();
                (*cloned).store(true, Ordering::SeqCst);
            })
            .expect("couldn't create stats thread.");
    }

    /// Parks a thread for THREAD_PARK_TIMEOUT or until unpark_thread is called
    /// depending on the number of remaining available threads
    pub fn park_thread(&self) {
        // We don't want to park threads while someone is asking for resources.
        if self.draining_parked_threads.load(Ordering::Acquire) {
            return;
        }
        let _ = self
            .parked_threads
            .push(std::thread::current())
            .map(|_| {
                debug!(
                    "Bastion-executor: parking the thread {:?}",
                    std::thread::current().id()
                );
                std::thread::park();
            })
            .map_err(|e| {
                debug!("Bastion-executor: park_thread: parking with timeout");
                self.park_timeout();
                e
            });
    }

    /// Parks a thread for THREAD_PARK_TIMEOUT or until it receives a wakeup signal
    pub fn park_timeout(&self) {
        debug!(
            "Bastion-executor: parking the thread {:?} for at least {}ms",
            std::thread::current().id(),
            THREAD_PARK_TIMEOUT.as_millis()
        );
        std::thread::park_timeout(THREAD_PARK_TIMEOUT);
    }

    /// Unparks all the threads from the queue
    pub fn unpark_all_threads(&self) {
        if !self
            .draining_parked_threads
            .compare_and_swap(false, true, Ordering::Release)
        {
            trace!("Bastion-executor: unparking all threads.");
            // Not draining yet, let's do it
            while let Ok(thread) = self.parked_threads.pop() {
                thread.unpark();
            }
            // We're done
            self.draining_parked_threads.store(false, Ordering::Release);
        }
    }

    /// Pops a thread from the parked_threads queue and unparks it
    pub fn unpark_thread(&self) {
        // We don't want to unpark a thread while someone is already doing it.
        if self.draining_parked_threads.load(Ordering::Acquire) {
            return;
        }
        if self.parked_threads.is_empty() {
            debug!("Bastion-executor: unpark_thread: no parked threads");
        } else {
            debug!("parked_threads: len is {}", self.parked_threads.len());
            let _ = self
                .parked_threads
                .pop()
                .map(|thread| {
                    debug!(
                        "Bastion-executor: unpark_thread: unparking {:?}",
                        thread.id()
                    );
                    thread.unpark()
                })
                .map_err(|e| {
                    debug!(
                        "Bastion-executor: unpark_thread: couldn't unpark thread - {}",
                        e
                    );
                });
        }
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
