//!
//! Module for gathering statistics about the run queues of the runtime
//!
//! Load balancer calculates sampled mean to provide average process execution amount
//! to all runtime.
//!
use crate::load_balancer;
use crate::placement;
use crossbeam_utils::sync::ShardedLock;
use fxhash::FxHashMap;
use lazy_static::*;
use std::thread;
use std::time::Duration;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::usize;
use std::mem::MaybeUninit;
/// Stats of all the smp queues.
pub trait SmpStats{
     fn store_load(&self, affinity: usize, load: usize);
     fn get_sorted_load(&self) -> Vec<(usize, usize)>;
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
                    if let Ok(mut stats) = load_balancer::stats().try_write() {
                        // Write latest downscaled mean to statistics
                        stats.mean_level = stats
                            .smp_queues
                            .values()
                            .sum::<usize>()
                            .wrapping_div(*core_retrieval());
                    }

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

///
/// Holding all statistics related to the run queue
///
/// Contains:
/// * Global run queue size
/// * Mean level of processes in the run queues
/// * SMP queue distributions
#[derive(Clone, Debug, Default)]
pub struct Stats {
    pub(crate) global_run_queue: usize,
    pub(crate) mean_level: usize,
    pub(crate) smp_queues: FxHashMap<usize, usize>,
}

impl SmpStats for ShardedLock<Stats>{

    fn store_load(&self, affinity: usize, load: usize){
        let mut stats = self.write().unwrap();
        stats.smp_queues.insert(affinity, load);
    }

    fn get_sorted_load(&self) -> Vec<(usize, usize)> {
        let stats = self.read().unwrap();

        // Collect all the stats.
        let mut core_vec: Vec<_> = stats
        .smp_queues
        .iter()
        .map(|(&x, &y)| (x, y))
        .collect::<Vec<(usize, usize)>>();
        
        // Sort to find the over loaded queue.
        core_vec.sort_by(|x, y| y.1.cmp(&x.1));
        core_vec
    }
}

/// Maximum number of core supported by mordern computers.
const MAX_CORE: usize = 40;

pub struct LockLessStats {
    smp_load: [AtomicUsize;MAX_CORE],
}

impl LockLessStats{
    /// new returns LockLessStats
    pub fn new(num_cores: usize)  -> LockLessStats{

        let smp_load: [AtomicUsize; MAX_CORE] = {
            let mut data: [MaybeUninit<AtomicUsize>; MAX_CORE] = unsafe{
                MaybeUninit::uninit().assume_init()
            };
            let mut i = 0;
            while i < MAX_CORE{
                if i < num_cores {
                    unsafe{std::ptr::write(data[i].as_mut_ptr(),AtomicUsize::new(0));}
                    i = i+1; 
                    continue;   
                }
                // MAX is for unused slot.
                unsafe{std::ptr::write(data[i].as_mut_ptr(),AtomicUsize::new(usize::MAX));}
                i = i+1;
            }
            unsafe {
                std::mem::transmute::<_, [AtomicUsize; MAX_CORE]>(data)
            }
        };
        LockLessStats{
            smp_load: smp_load,
        }
    }
}

unsafe impl Sync for LockLessStats{}
unsafe impl Send for LockLessStats{}

impl SmpStats for LockLessStats{

    fn store_load(&self, affinity: usize, load: usize){
        self.smp_load[affinity].store(load, Ordering::SeqCst);
    }

    fn get_sorted_load(&self) -> Vec<(usize, usize)>{
        let mut sorted_load = Vec::new();

        for (i, item) in self.smp_load.iter().enumerate(){
            let load = item.load(Ordering::SeqCst);
            if load == usize::MAX{
                break;
            }
            sorted_load.push((i, load));
        }
        sorted_load.sort_by(|x, y| y.1.cmp(&x.1));
        sorted_load
    }
}

unsafe impl Send for Stats {}
unsafe impl Sync for Stats {}

///
/// Static access to runtime statistics
#[inline]
pub fn stats() -> &'static ShardedLock<Stats> {
    lazy_static! {
        static ref LB_STATS: ShardedLock<Stats> = {
            let stats = Stats {
                global_run_queue: 0,
                mean_level: 0,
                smp_queues: FxHashMap::with_capacity_and_hasher(
                    placement::get_core_ids().unwrap().len(),
                    Default::default()
                )
            };

            // Start AMQL generator
            LoadBalancer::amql_generation();

            // Return stats
            ShardedLock::new(stats)
        };
    }
    &*LB_STATS
}


#[inline]
pub fn lockless_stats() -> &'static LockLessStats {
    lazy_static! {
        // hardcoding the number of cores.
        static ref LOCKLESS_STATS: LockLessStats = LockLessStats::new(12);
    }
    &*LOCKLESS_STATS
}

///
/// Retrieve core count for the runtime scheduling purposes
#[inline]
pub fn core_retrieval() -> &'static usize {
    lazy_static! {
        static ref CORE_COUNT: usize = { placement::get_core_ids().unwrap().len() };
    }

    &*CORE_COUNT
}
