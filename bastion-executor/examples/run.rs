use bastion_executor::prelude::*;
use lightproc::proc_stack::ProcStack;
use lightproc::proc_state::EmptyProcState;

fn main() {
    run(
        async {
            println!("Example execution");
            panic!("fault");
        },
        ProcStack::default().with_after_panic(|s: &mut EmptyProcState| {
            println!("after panic");
        }),
    );
}
