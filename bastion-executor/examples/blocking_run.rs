use bastion_executor::prelude::*;
use lightproc::proc_stack::ProcStack;


fn main() {
    run(
        async {
            println!("DATA");
            panic!("kaka");
        },
        ProcStack::default()
            .with_after_panic(|| {println!("after panic")}),
    );
}
