#![feature(test)]

extern crate test;

use bastion_executor::load_balancer;
use bastion_executor::prelude::spawn;
use futures_timer::Delay;
use lightproc::proc_stack::ProcStack;
use std::time::Duration;
use test::Bencher;

// Benchmark for a 10K burst task spawn
#[bench]
fn spawn_lot(b: &mut Bencher) {
    let core_count = *load_balancer::core_count();
    let proc_stack = ProcStack::default();
    b.iter(|| {
        let _ = (0..core_count)
            .map(|_| {
                spawn(
                    async {
                        let duration = Duration::from_millis(0);
                        Delay::new(duration).await;
                    },
                    proc_stack.clone(),
                )
            })
            .collect::<Vec<_>>();
    });
}

// Benchmark for a single task spawn
#[bench]
fn spawn_single(b: &mut Bencher) {
    let proc_stack = ProcStack::default();
    b.iter(|| {
        spawn(
            async {
                let duration = Duration::from_millis(0);
                Delay::new(duration).await;
            },
            proc_stack.clone(),
        );
    });
}
