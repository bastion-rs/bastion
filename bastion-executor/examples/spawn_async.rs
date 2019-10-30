use bastion_executor::prelude::*;
use lightproc::proc_stack::ProcStack;
use std::{thread, time};

fn main() {
    let pid = 1;
    let stack = ProcStack::default()
        .with_pid(pid)
        .with_after_panic(move || {println!("after panic {}", pid.clone())});

    let handle = spawn(async {
        panic!("test");
    }, stack);

    let pid = 2;
    let stack = ProcStack::default().with_pid(pid);

    run(
        async {
            handle.await;
        },
        stack.clone()
    );
}
