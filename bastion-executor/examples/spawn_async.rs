use bastion_executor::prelude::*;
use lightproc::proc_stack::{EmptyProcState, ProcStack};

fn main() {
    let pid = 1;
    let stack = ProcStack::default()
        .with_pid(pid)
        .with_after_panic(move |s: EmptyProcState| {
            println!("after panic {}", pid.clone());
            s
        });

    let handle = spawn(
        async {
            panic!("test");
        },
        stack,
    );

    let pid = 2;
    let stack = ProcStack::default().with_pid(pid);

    run(
        async {
            handle.await;
        },
        stack.clone(),
    );
}
