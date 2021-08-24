use std::{thread, time::Duration};

use bastion::prelude::*;

async fn child_task(ctx: BastionContext) -> Result<(), ()> {
    loop {
        MessageHandler::new(ctx.recv().await?)
            .on_tell(|_: DummyMessage, _| {
                ctx.read_data(|child_usize: Option<&usize>| {
                    let child_usize = child_usize.unwrap();

                    println!("My state says my usize is `{}`", child_usize);
                })
            })
            .on_fallback(|_, _| panic!("Unhandled message"));
    }
}

#[derive(Debug)]
struct DummyMessage;

fn main() {
    env_logger::init();

    Bastion::init();
    Bastion::start();

    let children = Bastion::children(|c| c.with_exec(child_task).with_data(42usize))
        .expect("Failed to spawn children");

    let child = &children.elems()[0];

    child.tell_anonymously(DummyMessage).unwrap();

    thread::sleep(Duration::from_secs(1));
}
