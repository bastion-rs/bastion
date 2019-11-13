//!
//! SMP parallelism based cache affine worker implementation
//!
//! This worker implementation relies on worker run queue statistics which are hold in the pinned global memory
//! where workload distribution calculated and amended to their own local queues.
use crate::load_balancer;
use crate::pool::{self, Pool};
use crate::run_queue::{Steal, Worker};
use lightproc::prelude::*;
use std::cell::{Cell, UnsafeCell};
use std::{iter, ptr};

///
/// Get the current process's stack
pub fn current() -> ProcStack {
    get_proc_stack(|proc| proc.clone())
        .expect("`proc::current()` called outside the context of the proc")
}

thread_local! {
    static STACK: Cell<*const ProcStack> = Cell::new(ptr::null_mut());
}

///
/// Set the current process's stack during the run of the future.
pub(crate) fn set_stack<F, R>(stack: *const ProcStack, f: F) -> R
where
    F: FnOnce() -> R,
{
    struct ResetStack<'a>(&'a Cell<*const ProcStack>);

    impl Drop for ResetStack<'_> {
        fn drop(&mut self) {
            self.0.set(ptr::null());
        }
    }

    STACK.with(|st| {
        st.set(stack);
        let _guard = ResetStack(st);

        f()
    })
}

pub(crate) fn get_proc_stack<F, R>(f: F) -> Option<R>
where
    F: FnOnce(&ProcStack) -> R,
{
    let res = STACK.try_with(|st| unsafe { st.get().as_ref().map(f) });

    match res {
        Ok(Some(val)) => Some(val),
        Ok(None) | Err(_) => None,
    }
}

thread_local! {
    static QUEUE: UnsafeCell<Option<Worker<LightProc>>> = UnsafeCell::new(None);
}

pub(crate) fn schedule(proc: LightProc) {
    QUEUE.with(|queue| {
        let local = unsafe { (*queue.get()).as_ref() };

        match local {
            None => pool::get().injector.push(proc),
            Some(q) => q.push(proc),
        }
    });

    pool::get().sleepers.notify_one();
}

///
/// Fetch the process from the run queue.
/// Does the work of work-stealing if process doesn't exist in the local run queue.
pub fn fetch_proc(affinity: usize) -> Option<LightProc> {
    let pool = pool::get();

    QUEUE.with(|queue| {
        let local = unsafe { (*queue.get()).as_ref().unwrap() };
        local.pop().or_else(|| affine_steal(pool, local, affinity))
    })
}

fn affine_steal(pool: &Pool, local: &Worker<LightProc>, affinity: usize) -> Option<LightProc> {
    // Pop a task from the local queue, if not empty.
    local.pop().or_else(|| {
        // Otherwise, we need to look for a task elsewhere.
        iter::repeat_with(|| match load_balancer::stats().try_read() {
            Ok(stats) => {
                // Collect run queue statistics
                let mut core_vec: Vec<_> = stats
                    .smp_queues
                    .iter()
                    .map(|(&x, &y)| (x, y))
                    .collect::<Vec<(usize, usize)>>();

                // Sort cores and their run queue sizes.
                //
                // This sort will be ordered by descending order
                // so we can pick up from the most overloaded queue.
                core_vec.sort_by(|x, y| y.1.cmp(&x.1));

                // First try to get procs from global queue
                pool.injector.steal_batch_and_pop(&local).or_else(|| {
                    match core_vec.get(0) {
                        Some((core, _)) => {
                            // If affinity is the one with the highest let other's do the stealing
                            if *core == affinity {
                                Steal::Retry
                            } else {
                                // Try iterating through biggest to smallest
                                core_vec
                                    .iter()
                                    .map(|s| {
                                        // Steal the mean amount to balance all queues considering incoming workloads
                                        // Otherwise do an ignorant steal (which is going to be useless)
                                        if stats.mean_level > 0 {
                                            pool.stealers
                                                .get(s.0)
                                                .unwrap()
                                                .steal_batch_and_pop_with_amount(
                                                    &local,
                                                    stats.mean_level,
                                                )
                                        } else {
                                            pool.stealers
                                                .get(s.0)
                                                .unwrap()
                                                .steal_batch_and_pop(&local)
                                        }
                                    })
                                    .collect()
                            }
                        }
                        _ => Steal::Retry,
                    }
                })
            }
            Err(_) => Steal::Retry,
        })
        // Loop while no task was stolen and any steal operation needs to be retried.
        .find(|s| !s.is_retry())
        // Extract the stolen task, if there is one.
        .and_then(|s| s.success())
    })
}

pub(crate) fn stats_generator(affinity: usize, local: &Worker<LightProc>) {
    loop {
        if let Ok(mut stats) = load_balancer::stats().try_write() {
            stats
                .smp_queues
                .insert(affinity, local.worker_run_queue_size());
            break;
        }
    }
}

pub(crate) fn main_loop(affinity: usize, local: Worker<LightProc>) {
    QUEUE.with(|queue| unsafe { *queue.get() = Some(local) });

    loop {
        QUEUE.with(|queue| {
            let local = unsafe { (*queue.get()).as_ref().unwrap() };
            stats_generator(affinity, local);
        });

        match fetch_proc(affinity) {
            Some(proc) => set_stack(proc.stack(), || proc.run()),
            None => pool::get().sleepers.wait(),
        }
    }
}
