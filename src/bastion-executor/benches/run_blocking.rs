#![feature(test)]

extern crate test;

use bastion_executor::blocking;
use bastion_executor::run::run;
use futures::future::join_all;
use lightproc::proc_stack::ProcStack;
use std::thread;
use std::time::Duration;
use test::Bencher;

#[cfg(feature = "tokio-runtime")]
mod tokio_benchs {
    use super::*;
    #[bench]
    fn blocking(b: &mut Bencher) {
        tokio_test::block_on(async { _blocking(b) });
    }
    #[bench]
    fn blocking_single(b: &mut Bencher) {
        tokio_test::block_on(async {
            _blocking_single(b);
        });
    }
}

#[cfg(not(feature = "tokio-runtime"))]
mod no_tokio_benchs {
    use super::*;
    #[bench]
    fn blocking(b: &mut Bencher) {
        _blocking(&mut b);
    }
    #[bench]
    fn blocking_single(b: &mut Bencher) {
        _blocking_single(&mut b);
    }
}

// Benchmark for a 10K burst task spawn
fn _blocking(b: &mut Bencher) {
    b.iter(|| {
        (0..10_000)
            .map(|_| {
                blocking::spawn_blocking(
                    async {
                        let duration = Duration::from_millis(1);
                        thread::sleep(duration);
                    },
                    ProcStack::default(),
                )
            })
            .collect::<Vec<_>>()
    });
}

// Benchmark for a single blocking task spawn
fn _blocking_single(b: &mut Bencher) {
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
