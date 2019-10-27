use super::distributor::Distributor;
use super::run_queue::{Worker, Injector, Stealer};
use lazy_static::*;
use lightproc::prelude::*;
use super::sleepers::Sleepers;

pub struct Pool {
    pub injector: Injector<LightProc>,
    pub stealers: Vec<Stealer<LightProc>>,
    pub sleepers: Sleepers,
}

impl Pool {
    /// Error recovery for the fallen threads
    pub fn recover_async_thread() {
        unimplemented!()
    }

    pub fn fetch_proc(&self, local: &Worker<LightProc>) -> Option<LightProc> {
        // Pop only from the local queue with full trust
        local.pop()
    }
}

#[inline]
pub fn get() -> &'static Pool {
    lazy_static! {
        static ref POOL: Pool = {
            let distributor = Distributor::new();
            let (stealers, workers) = distributor.assign(|| {
                println!("1,2,3");
            });

            Pool {
                injector: Injector::new(),
                stealers,
                sleepers: Sleepers::new(),
            }
        };
    }
    &*POOL
}
