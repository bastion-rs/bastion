use bastion_executor::prelude::*;
use lightproc::proc_stack::ProcStack;

fn main() {
    run(
        async {
            println!("Example execution");
            panic!("fault");
        },
        ProcStack::default().with_after_panic(|| println!("after panic")),
    );
}
