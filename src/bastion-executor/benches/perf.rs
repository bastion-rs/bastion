#![feature(test)]

extern crate test;

use bastion_executor::prelude::*;
use lightproc::proc_stack::ProcStack;
use test::{black_box, Bencher};

#[bench]
fn increment(b: &mut Bencher) {
    let mut sum = 0;

    b.iter(|| {
        run(
            async {
                (0..10_000_000).for_each(|_| {
                    sum += 1;
                });
            },
            ProcStack::default(),
        );
    });

    black_box(sum);
}
