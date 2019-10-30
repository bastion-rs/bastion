use super::placement;
use super::placement::CoreId;
use super::run_queue::{Stealer, Worker};

use lightproc::prelude::*;

use crate::worker;
use std::thread;

pub(crate) struct Distributor {
    pub round: usize,
    pub last_dead: usize,
    pub cores: Vec<CoreId>,
}

impl Distributor {
    pub fn new() -> Self {
        Distributor {
            round: 0_usize,
            last_dead: usize::max_value(),
            cores: placement::get_core_ids().expect("Core mapping couldn't be fetched"),
        }
    }

    pub fn assign(mut self) -> Vec<Stealer<LightProc>> {
        let mut stealers = Vec::<Stealer<LightProc>>::new();

        for core in self.cores {
            self.round = core.id;

            let wrk = Worker::new_fifo();
            stealers.push(wrk.stealer());

            thread::Builder::new()
                .name("bastion-async-thread".to_string())
                .spawn(move || {
                    // affinity assignment
                    placement::set_for_current(core);

                    // actual execution
                    worker::main_loop(core.id.clone(), wrk);
                })
                .expect("cannot start the thread for running proc");
        }

        stealers
    }
}
