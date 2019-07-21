use bastion::bastion::Bastion;
use bastion::child::Message;
use bastion::context::BastionContext;
use bastion::supervisor::SupervisionStrategy;
use std::{fs, thread};

fn main() {
    Bastion::platform();

    let message = "HOPS : ".to_string();
    let message2 = "Some Other Message".to_string();

    Bastion::supervisor("background-worker", "new-system")
        .strategy(SupervisionStrategy::OneForAll)
        .children(
            |p: BastionContext, message: Box<dyn Message>| {
                // Message can be casted and reused here.
                let mut i = 0;
                loop {
                    i = i + 1;
                    let received_msg: String = message
                        .as_any()
                        .downcast_ref::<String>()
                        .unwrap()
                        .to_string();
                    let new_msg = format!("{}{} ", received_msg, i);

                    let tx = p.bcast_tx.as_ref().unwrap().clone();

                    tx.send(Box::new(new_msg));

                    let rx = p.bcast_rx.clone().unwrap();
                    if let Ok(message) = rx.try_recv() {
                        let msg: String = message
                            .as_any()
                            .downcast_ref::<String>()
                            .unwrap()
                            .to_string();
                        println!("Cooperatively assembled message :: {}", msg);
                    }

                    // Hook to rebind to the system.
                    p.clone().hook();
                }
            },
            message,
            2_i32,
        )
        .launch();

    Bastion::start()
}
