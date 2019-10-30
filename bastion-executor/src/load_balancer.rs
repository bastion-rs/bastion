use super::pool;
use super::run_queue::Worker;
use super::placement;
use lazy_static::*;
use lightproc::lightproc::LightProc;

use std::{thread, time};
use std::sync::atomic::AtomicUsize;
use std::collections::HashMap;
use std::sync::RwLock;
use rustc_hash::FxHashMap;

const SIXTY_MILLIS: time::Duration = time::Duration::from_millis(60);

pub struct LoadBalancer();

impl LoadBalancer {
    pub fn sample(self, workers: Vec<Worker<LightProc>>) -> LoadBalancer {
        thread::Builder::new()
            .name("load-balancer-thread".to_string())
            .spawn(move || {
                loop {


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

        self
    }
}

#[derive(Clone)]
pub struct Stats {
    global_run_queue: usize,
    smp_queues: FxHashMap<usize, usize>,
}

unsafe impl Send for Stats {}
unsafe impl Sync for Stats {}

#[inline]
pub fn stats() -> &'static RwLock<Stats> {
    lazy_static! {
        static ref LB_STATS: RwLock<Stats> = {
            let stats = Stats {
                global_run_queue: 0,
                smp_queues: FxHashMap::with_capacity_and_hasher(
                    placement::get_core_ids().unwrap().len(),
                    Default::default()
                )
            };

            RwLock::new(stats)
        };
    }
    &*LB_STATS
}
