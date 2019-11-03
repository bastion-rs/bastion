use super::distributor::Distributor;

use super::run_queue::{Injector, Stealer, Worker};
use super::sleepers::Sleepers;
use super::worker;
use lazy_static::*;
use lightproc::prelude::*;
use std::future::Future;

use crate::load_balancer;

pub fn spawn<F, T>(future: F, stack: ProcStack) -> RecoverableHandle<T>
where
    F: Future<Output = T> + Send + 'static,
    T: Send + 'static,
{
    self::get().spawn(future, stack)
}

pub struct Pool {
    pub injector: Injector<LightProc>,
    pub stealers: Vec<Stealer<LightProc>>,
    pub sleepers: Sleepers,
}

impl Pool {
    /// Error recovery for the fallen threads
    pub fn recover_async_thread() {
        // FIXME: Do recovery for fallen worker threads
        unimplemented!()
    }

    pub fn fetch_proc(&self, affinity: usize, local: &Worker<LightProc>) -> Option<LightProc> {
        if let Ok(mut stats) = load_balancer::stats().try_write() {
            stats
                .smp_queues
                .insert(affinity, local.worker_run_queue_size());
        }

        if let Ok(stats) = load_balancer::stats().try_read() {
            if local.worker_run_queue_size() == 0 {
                while let Some(proc) = self.injector.steal_batch_and_pop(local).success() {
                    return Some(proc);
                }
            } else {
                let affine_core = *stats
                    .smp_queues
                    .iter()
                    .max_by_key(|&(_core, stat)| stat)
                    .unwrap()
                    .1;
                let stealer = self.stealers.get(affine_core).unwrap();
                if let Some(amount) = stealer.run_queue_size().checked_sub(stats.mean_level) {
                    if let Some(proc) = stealer
                        .steal_batch_and_pop_with_amount(local, amount.wrapping_add(1))
                        .success()
                    {
                        return Some(proc);
                    }
                }
            }
        }

        // Pop only from the local queue with full trust
        local.pop()
    }

    pub fn spawn<F, T>(&self, future: F, stack: ProcStack) -> RecoverableHandle<T>
    where
        F: Future<Output = T> + Send + 'static,
        T: Send + 'static,
    {
        // Log this `spawn` operation.
        let _child_id = stack.get_pid() as u64;
        let _parent_id = worker::get_proc_stack(|t| t.get_pid() as u64).unwrap_or(0);

        let (task, handle) = LightProc::recoverable(future, worker::schedule, stack);
        task.schedule();
        handle
    }
}

#[inline]
pub fn get() -> &'static Pool {
    lazy_static! {
        static ref POOL: Pool = {
            let distributor = Distributor::new();
            let stealers = distributor.assign();

            Pool {
                injector: Injector::new(),
                stealers,
                sleepers: Sleepers::new(),
            }
        };
    }
    &*POOL
}
