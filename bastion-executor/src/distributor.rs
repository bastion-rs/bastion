//!
//! Cache affine thread pool distributor
//!
//! Distributor provides a fair distribution of threads and pinning them to cores for fair execution.
//! It assigns threads in round-robin fashion to all cores.
use crate::placement::{self, CoreId};
use crate::run_queue::{Stealer, Worker};
use crate::worker;
use lightproc::prelude::*;
use std::thread;

pub(crate) struct Distributor {
    pub(crate) cores: Vec<CoreId>,
}

impl Distributor {
    pub(crate) fn new() -> Self {
        Distributor {
            cores: placement::get_core_ids().expect("Core mapping couldn't be fetched"),
        }
    }

    pub(crate) fn assign(self) -> Vec<Stealer<LightProc>> {
        let mut stealers = Vec::<Stealer<LightProc>>::new();

        for core in self.cores {
            let wrk = Worker::new_fifo();
            stealers.push(wrk.stealer());

            thread::Builder::new()
                .name("bastion-async-thread".to_string())
                .spawn(move || {
                    // affinity assignment
                    placement::set_for_current(core);

                    // run initial stats generation for cores
                    worker::stats_generator(core.id.clone(), &wrk);
                    // actual execution
                    worker::main_loop(core.id.clone(), wrk);
                })
                .expect("cannot start the thread for running proc");
        }

        stealers
    }
}
