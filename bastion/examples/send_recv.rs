use bastion::prelude::*;
use futures::pending;

#[runtime::main]
async fn main() {
    Bastion::init();

    Bastion::children(
        |ctx: BastionContext| {
            async move {
                let id = ctx.id();
                let hello_world = "Hello World!".to_string();

                // This is going to return None.
                let try_recv = ctx.try_recv().await;
                println!("try_recv.is_some() == {}", try_recv.is_some());

                ctx.send_msg(id, Box::new(hello_world));

                // This is going to return Ok(Box("Hello World!"))
                let recv = ctx.recv().await;
                println!("recv.is_ok() == {}", recv.is_ok());

                Ok(())
            }
            .into()
        },
        1,
    );

    Bastion::start();

    loop {}
}
