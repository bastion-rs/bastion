use bastion::prelude::*;

fn main() {
    Bastion::init();

    Bastion::children(
        |ctx: BastionContext| {
            async move {
                // This will return None.
                let try_recv = ctx.try_recv().await;
                println!("try_recv.is_some() == {}", try_recv.is_some()); // false

                ctx.send_msg(ctx.as_ref(), Box::new("Hello World!")).ok();

                // This will return Ok(Box("Hello World!")).
                let recv = ctx.recv().await;
                println!("recv.is_ok() == {}", recv.is_ok()); // true

                // Panicking will restart the children group.
                panic!("Oh no!");
            }
            .into()
        },
        1,
    ).expect("Couldn't start a new children group.");

    Bastion::start();
    Bastion::block_until_stopped();
}
