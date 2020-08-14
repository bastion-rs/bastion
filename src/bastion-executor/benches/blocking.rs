#![feature(test)]

extern crate test;

use bastion_executor::blocking;
use lightproc::proc_stack::ProcStack;
use std::thread;
use std::time::Duration;
use test::Bencher;

// Benchmark for a 10K burst task spawn
#[bench]
fn blocking(b: &mut Bencher) {
    b.iter(|| {
        (0..100)
            .map(|_| {
                blocking::spawn_blocking(
                    async {
                        let duration = Duration::from_millis(0);
                        thread::sleep(duration);
                    },
                    ProcStack::default(),
                )
            })
            .collect::<Vec<_>>()
    });
}

// Benchmark for a single blocking task spawn
#[bench]
fn blocking_single(b: &mut Bencher) {
    b.iter(|| {
        blocking::spawn_blocking(
            async {
                let duration = Duration::from_millis(0);
                thread::sleep(duration);
            },
            ProcStack::default(),
        )
    });
}
