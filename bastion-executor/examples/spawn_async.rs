use bastion_executor::prelude::*;
use lightproc::proc_stack::ProcStack;
use std::{thread, time};

const SIXTY_MILLIS: time::Duration = time::Duration::from_millis(4000);

fn main() {
    let stack = ProcStack::default()
        .with_pid(1)
        .with_after_panic(|| {println!("after panic")});

    let handle = spawn(async {
        panic!("test");
    }, stack);

    let stack = ProcStack::default()
        .with_pid(2)
        .with_after_panic(|| {println!("after panic")});

    run(
        async {
            handle.await
        },
        stack.clone()
    );
}
