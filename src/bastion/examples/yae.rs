use bastion::prelude::*;
use futures::future::lazy;

#[derive(Debug)]
struct DemoMessage;

fn main() {
    env_logger::init();
    Bastion::init();
    Bastion::start();

    Bastion::supervisor(|sp| {
        let sp = sp.with_strategy(SupervisionStrategy::OneForOne);
        let receiver_ref = sp
            .children_ref(move |children| {
                children
                    .with_name("Receiver")
                    .with_exec(move |ctx| async move {
                        loop {
                            let maybe_msg = ctx.try_recv().await;
                            if let Some(msg) = maybe_msg {
                                msg! { msg,
                                    msg: DemoMessage => {
                                        eprintln!("DemoMessage!");
                                        Bastion::stop();
                                    };
                                    _:_ => panic!("Invalid message received");
                                }
                            } else {
                                eprintln!("Idling around");
                            }
                        }
                    })
            })
            .elems()[0]
            .addr();
        sp.children_ref(move |children| {
            children.with_name("Sender").with_exec(move |ctx| {
                let receiver_ref = receiver_ref.clone();
                async move {
                    loop {
                        lazy(|_| {
                            eprintln!("Sending message");
                            ctx.tell(&receiver_ref, DemoMessage).unwrap();
                        })
                        .await;
                    }
                    Ok(())
                }
            })
        });
        sp
    })
    .unwrap();

    Bastion::block_until_stopped();
}
