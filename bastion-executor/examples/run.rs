use bastion_executor::prelude::*;
use lightproc::proc_stack::{EmptyProcState, ProcStack};

fn main() {
    run(
        async {
            println!("Example execution");
            panic!("fault");
        },
        ProcStack::default().with_after_panic(|s: EmptyProcState| {
            println!("after panic");
            s
        }),
    );
}
