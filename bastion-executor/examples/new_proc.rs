use bastion_executor::prelude::*;
use lightproc::proc_stack::ProcStack;

fn main() {
    spawn(
        async {
            println!("DATA");
        },
        ProcStack::default(),
    );
}
