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
use tracing::Level;

// Benchmark for a 10K burst task spawn
#[bench]
fn run_blocking(b: &mut Bencher) {
    b.iter(|| {
        let handles = (0..10_000)
            .map(|_| {
                blocking::spawn_blocking(
                    async {
                        let duration = Duration::from_millis(0);
                        thread::sleep(duration);
                    },
                    ProcStack::default(),
                )
            })
            .collect::<Vec<_>>();

        run(join_all(handles), ProcStack::default())
    });
}

// Benchmark for a single blocking task spawn
#[bench]
fn run_blocking_single(b: &mut Bencher) {
    b.iter(|| {
        run(
            blocking::spawn_blocking(
                async {
                    let duration = Duration::from_millis(0);
                    thread::sleep(duration);
                },
                ProcStack::default(),
            ),
            ProcStack::default(),
        )
    });
}
