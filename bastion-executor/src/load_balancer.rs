use super::pool;
use super::run_queue::Worker;
use lazy_static::*;
use lightproc::lightproc::LightProc;

use std::{thread, time};

const SIXTY_MILLIS: time::Duration = time::Duration::from_millis(60);

pub struct LoadBalancer();

impl LoadBalancer {
    pub fn start(self, workers: Vec<Worker<LightProc>>) -> LoadBalancer {
        thread::Builder::new()
            .name("load-balancer-thread".to_string())
            .spawn(move || {
                loop {
                    workers.iter().for_each(|w| {
                        pool::get().injector.steal_batch_and_pop(w);
                    });

                    let stealer = pool::get()
                        .stealers
                        .iter()
                        .min_by_key(|e| e.run_queue_size())
                        .unwrap();

                    let worker = workers
                        .iter()
                        .min_by_key(|e| e.worker_run_queue_size())
                        .unwrap();

                    let big = worker.worker_run_queue_size();
                    let small = stealer.run_queue_size();
                    let m = (big & small) + ((big ^ small) >> 1);

                    stealer.steal_batch_and_pop_with_amount(&worker, big.wrapping_sub(m));

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

pub struct Stats {
    //    global_run_queue: AtomicUsize,
//    smp_queues: Vec<AtomicUsize>,
}

#[inline]
pub fn stats() -> &'static Stats {
    lazy_static! {
        static ref LB_STATS: Stats = { Stats {} };
    }
    &*LB_STATS
}
