use lazy_static::*;
use std::{thread, time};

const SIXTY_MILLIS: time::Duration = time::Duration::from_millis(60);

pub struct LoadBalancer();

impl LoadBalancer {
    pub fn trigger() {
        unimplemented!()
    }
}

#[inline]
pub(crate) fn launch() -> &'static LoadBalancer {
    lazy_static! {
        static ref LOAD_BALANCER: LoadBalancer = {
            thread::Builder::new()
                .name("load-balancer-thread".to_string())
                .spawn(|| {

                    // General suspending is equal to cache line size in ERTS
                    // https://github.com/erlang/otp/blob/master/erts/emulator/beam/erl_process.c#L10887
                    // https://github.com/erlang/otp/blob/ea7d6c39f2179b2240d55df4a1ddd515b6d32832/erts/emulator/beam/erl_thr_progress.c#L237
                    thread::sleep(SIXTY_MILLIS)
                })
                .expect("load-balancer couldn't start");

            LoadBalancer()
        };
    }
    &*LOAD_BALANCER
}
