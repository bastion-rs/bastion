use bastion::prelude::*;

fn main() {
    Bastion::init();

    Bastion::children(
        |ctx: BastionContext| {
            async move {
                // This will return None.
                let try_recv = ctx.try_recv().await;
                println!("try_recv.is_some() == {}", try_recv.is_some()); // false

                ctx.current().send_msg(Box::new("Hello World!")).ok();

                message! { ctx.recv().await?,
                    msg: &'static str => {
                        println!(r#"msg == "Hello World!" => {}"#, msg == &"Hello World!"); // true
                    },
                    // This won't happen because we know that this
                    // example only sends a `&'static str`...
                    _ => unreachable!(),
                }

                // Panicking will restart the children group.
                panic!("Oh no!");
            }
            .into()
        },
        1,
    )
    .expect("Couldn't start a new children group.");

    Bastion::start();
    Bastion::block_until_stopped();
}
