use std::{thread, time::Duration};

use bastion::prelude::*;

async fn child_task(ctx: BastionContext) -> Result<(), ()> {
    loop {
        MessageHandler::new(ctx.recv().await?)
            .on_tell(|_: DummyMessage, _| {
                ctx.write_data(|value| {
                    value.map(|v: &usize| {
                        println!("My counter is at `{}`. Let's increment it!", v);
                        v + 1
                    })
                });
            })
            .on_fallback(|_, _| panic!("Unhandled message"));
    }
}

#[derive(Debug)]
struct DummyMessage;

// $ cargo r --example with_data
// My counter is at `42`. Let's increment it!
// My counter is at `43`. Let's increment it!
// My counter is at `44`. Let's increment it!
fn main() {
    env_logger::init();

    Bastion::init();
    Bastion::start();

    let children = Bastion::children(|c| c.with_exec(child_task).with_data(42usize))
        .expect("Failed to spawn children");

    let child = &children.elems()[0];

    child.tell_anonymously(DummyMessage).unwrap();
    child.tell_anonymously(DummyMessage).unwrap();
    child.tell_anonymously(DummyMessage).unwrap();

    thread::sleep(Duration::from_secs(1));
}
