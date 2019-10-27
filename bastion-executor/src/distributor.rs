use super::placement;
use super::placement::CoreId;
use std::thread;
use super::run_queue::{Worker, Stealer};
use lightproc::prelude::*;

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

    pub fn assign<P>(mut self, thunk: P) -> Vec<Stealer<LightProc>>
    where
        P: Fn() + Send + Sync + Copy + 'static,
    {
        let mut stealers = Vec::<Stealer<LightProc>>::new();
        for core in self.cores {
            self.round = core.id;

            let worker = Worker::new_fifo();
            stealers.push(worker.stealer());

            thread::Builder::new()
                .name("bastion-async-thread".to_string())
                .spawn(move || {
                    // affinity assignment
                    placement::set_for_current(core);

                    // actual execution
                    thunk();
                })
                .expect("cannot start the thread for running proc");
        }

        stealers
    }
}
