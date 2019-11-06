use lightproc::proc_stack::ProcStack;

#[test]
fn stack_copy() {
    let stack = ProcStack::default().with_pid(12).with_after_panic(|| {
        println!("After panic!");
    });

    let stack2 = stack.clone();

    assert_eq!(stack2.get_pid(), 12);
    stack2.after_panic.unwrap()();
}
