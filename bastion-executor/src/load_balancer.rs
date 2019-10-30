

use super::placement;
use lazy_static::*;


use std::{thread, time};


use crossbeam_utils::sync::ShardedLock;
use rustc_hash::FxHashMap;
use super::load_balancer;

const SIXTY_MILLIS: time::Duration = time::Duration::from_millis(60);

pub struct LoadBalancer();

impl LoadBalancer {
    pub fn sample() {
        thread::Builder::new()
            .name("load-balancer-thread".to_string())
            .spawn(move || {
                loop {
                    let mut m = 0_usize;
                    if let Ok(stats) = load_balancer::stats().try_read() {
                        m = stats.smp_queues.values().sum::<usize>()
                            .wrapping_div(placement::get_core_ids().unwrap().len());
                    }

                    if let Ok(mut stats) = load_balancer::stats().try_write() {
                        stats.mean_level = m;
                    }

                    // General suspending is equal to cache line size in ERTS
                    // https://github.com/erlang/otp/blob/master/erts/emulator/beam/erl_process.c#L10887
                    // https://github.com/erlang/otp/blob/ea7d6c39f2179b2240d55df4a1ddd515b6d32832/erts/emulator/beam/erl_thr_progress.c#L237
                    // thread::sleep(SIXTY_MILLIS);
                    (0..64).for_each(|_| unsafe {
                        asm!("NOP");
                    })
                }
            })
            .expect("load-balancer couldn't start");
    }
}

#[derive(Clone)]
pub struct Stats {
    pub(crate) global_run_queue: usize,
    pub(crate) mean_level: usize,
    pub(crate) smp_queues: FxHashMap<usize, usize>,
}

unsafe impl Send for Stats {}
unsafe impl Sync for Stats {}

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

            // Start sampler
            LoadBalancer::sample();

            // Return stats
            ShardedLock::new(stats)
        };
    }
    &*LB_STATS
}
