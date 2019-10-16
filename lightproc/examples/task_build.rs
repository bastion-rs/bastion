use lightproc::prelude::*;
use lightproc::stack::ProcStack;

fn main() {
    let lp = LightProc::<()>::new().with_future(async move {
        println!("test");
    });
    dbg!(lp);

    let lp2 = LightProc::<()>::new()
        .with_future(async move {
            println!("future");
        })
        .with_schedule(|t| {
            println!("scheduler");
        });
    dbg!(lp2);

    let lp3 = LightProc::<()>::new()
        .with_schedule(|t| {
            println!("scheduler");
        });
    dbg!(lp3);

    let lp4 = LightProc::<ProcStack>::new()
        .with_future(async move {
            println!("future");
        })
        .with_schedule(|t| {
            println!("scheduler");
        })
        .with_stack(ProcStack::default());
    dbg!(lp4);
}
