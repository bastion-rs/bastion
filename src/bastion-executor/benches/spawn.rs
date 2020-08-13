#![feature(test)]

extern crate test;

use bastion_executor::prelude::spawn;
use bastion_executor::run::run;
use futures::future::join_all;
use futures_timer::Delay;
use lightproc::proc_stack::ProcStack;
use lightproc::recoverable_handle::RecoverableHandle;
use std::time::Duration;
use test::Bencher;

// Benchmark for a 10K burst task spawn
#[bench]
fn spawn_lot(b: &mut Bencher) {
    let proc_stack = ProcStack::default();
    b.iter(|| {
        let handles = (0..10_000)
            .map(|_| {
                spawn(
                    async {
                        let duration = Duration::from_millis(0);
                        Delay::new(duration).await;
                    },
                    proc_stack.clone(),
                )
            })
            .collect::<Vec<RecoverableHandle<()>>>();

        run(join_all(handles), proc_stack.clone());
    });
}

// Benchmark for a single blocking task spawn
#[bench]
fn spawn_single(b: &mut Bencher) {
    let proc_stack = ProcStack::default();
    b.iter(|| {
        let handle = spawn(
            async {
                let duration = Duration::from_millis(0);
                Delay::new(duration).await;
            },
            proc_stack.clone(),
        );
        run(handle, proc_stack.clone())
    });
}
