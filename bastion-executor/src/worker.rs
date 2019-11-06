use std::cell::{UnsafeCell, Cell};
use std::ptr;

use super::pool;
use super::run_queue::Worker;
use lightproc::prelude::*;
use core::iter;
use crate::load_balancer;
use std::iter::repeat_with;
use crate::pool::Pool;
use std::sync::atomic::Ordering;
use std::sync::atomic;

pub fn current() -> ProcStack {
    get_proc_stack(|proc| proc.clone())
        .expect("`proc::current()` called outside the context of the proc")
}

thread_local! {
    static STACK: Cell<*const ProcStack> = Cell::new(ptr::null_mut());
}

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

pub fn fetch_proc(affinity: usize) -> Option<LightProc> {
    let pool = pool::get();

    QUEUE.with(|queue| {
        let local = unsafe { (*queue.get()).as_ref().unwrap() };

        stats_generator(affinity, local);

        // Pop only from the local queue with full trust
        local.pop().or_else(|| {
            match local.worker_run_queue_size() {
                x if x == 0 => {
                    if pool.injector.is_empty() {
                        affine_steal(pool, local)
                    } else {
                        pool.injector.steal_batch_and_pop(local).success()
                    }
                },
                _ => {
                    affine_steal(pool, local)
                }
            }
        })
    })
}

fn affine_steal(pool: &Pool, local: &Worker<LightProc>) -> Option<LightProc> {
    match load_balancer::stats().try_read() {
        Ok(stats) => {
            let affine_core = *stats
                .smp_queues
                .iter()
                .max_by_key(|&(_core, stat)| stat)
                .unwrap()
                .0;

            let default = || {
                pool.injector.steal_batch_and_pop(local).success()
            };

            pool.stealers.get(affine_core)
                .map_or_else(default, |stealer| {
                    stealer.run_queue_size().checked_sub(stats.mean_level)
                        .map_or_else(default, |amount| {
                            amount.checked_sub(1)
                                .map_or_else(default, |possible| {
                                    if possible != 0 {
                                        stealer.steal_batch_and_pop_with_amount(local, possible).success()
                                    } else {
                                        default()
                                    }
                                })
                        })
                })
        },
        Err(_) => {
            pool.injector.steal_batch_and_pop(local).success()
        }
    }
}

fn stats_generator(affinity: usize, local: &Worker<LightProc>) {
    if let Ok(mut stats) = load_balancer::stats().try_write() {
        stats
            .smp_queues
            .insert(affinity, local.worker_run_queue_size());
    }
}

pub(crate) fn main_loop(affinity: usize, local: Worker<LightProc>) {
    QUEUE.with(|queue| unsafe { *queue.get() = Some(local) });

    loop {
        match fetch_proc(affinity) {
            Some(proc) => set_stack(proc.stack(), || proc.run()),
            None => {
                QUEUE.with(|queue| {
                    let local = unsafe { (*queue.get()).as_ref().unwrap() };
                    stats_generator(affinity, local);
                });
                pool::get().sleepers.wait()
            },
        }
    }
}
