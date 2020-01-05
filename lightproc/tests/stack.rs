use lightproc::proc_stack::{EmptyProcState, ProcStack};

#[test]
fn stack_copy() {
    let stack = ProcStack::default()
        .with_pid(12)
        .with_after_panic(|s: EmptyProcState| {
            println!("After panic!");
            s
        });

    let stack2 = stack.clone();

    assert_eq!(stack2.get_pid(), 12);
}
