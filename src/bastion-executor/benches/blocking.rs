#![feature(test)]

extern crate test;

use bastion_executor::blocking;
use bastion_executor::run::run;
use futures::future::join_all;
use lightproc::proc_stack::ProcStack;
use lightproc::recoverable_handle::RecoverableHandle;
use std::thread;
use std::time::Duration;
use test::Bencher;

// Benchmark for a 10K burst task spawn
#[bench]
fn blocking(b: &mut Bencher) {
    b.iter(|| {
        let handles = (0..10_000)
            .map(|_| {
                blocking::spawn_blocking(
                    async {
                        let duration = Duration::from_millis(1);
                        thread::sleep(duration);
                    },
                    ProcStack::default(),
                )
            })
            .collect::<Vec<RecoverableHandle<()>>>();

        run(join_all(handles), ProcStack::default());
    });
}

// Benchmark for a single blocking task spawn
#[bench]
fn blocking_single(b: &mut Bencher) {
    b.iter(|| {
        blocking::spawn_blocking(
            async {
                let duration = Duration::from_millis(1);
                thread::sleep(duration);
            },
            ProcStack::default(),
        )
    });
}
