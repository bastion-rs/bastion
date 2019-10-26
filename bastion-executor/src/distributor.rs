
use std::thread;
use crate::placement;
use crate::placement::CoreId;


pub(crate) struct Distributor {
    pub round: usize,
    pub last_dead: usize,
    pub cores: Vec<CoreId>
}

impl Distributor {
    pub fn new() -> Self {
        Distributor {
            round: 0_usize,
            last_dead: usize::max_value(),
            cores: placement::get_core_ids()
                .expect("Core mapping couldn't be fetched")
        }
    }

    pub fn assign<P>(mut self, thunk: P)
    where
        P: Fn() + Send + Sync + Copy + 'static
    {
        for core in self.cores {
            self.round = core.id;

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
    }
}
