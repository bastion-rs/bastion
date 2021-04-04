use bastion::children::RecipientTarget;
use bastion::{prelude::*, recipient::Recipient};
use std::sync::Arc;
use tracing::Level;

fn main() {
    // Initialize tracing logger
    // so we get nice output on the console.
    let subscriber = tracing_subscriber::fmt()
        .with_max_level(Level::WARN)
        .finish();
    tracing::subscriber::set_global_default(subscriber).unwrap();

    // We need bastion to run our program
    Bastion::init();
    // We create the supervisor and we add both groups on it
    Bastion::supervisor(receiver_supervisor).expect("Couldn't create supervisor chain.");
    // We are starting the Bastion program now
    Bastion::start();

    let recipient = Recipient::named("Receiver");
    let data_to_send: Vec<&str> = vec!["data_1", "data_2", "data_3", "data_4", "data_5"];

    for data in data_to_send {
        std::thread::sleep(std::time::Duration::from_secs(2));
        tracing::warn!("sending message");
        // We broadcast the message containing the data to the defined target
        dbg!(&recipient.tell(data));
    }

    tracing::warn!("DONE!");
    Bastion::stop();

    // We are waiting until the Bastion has stopped or got killed
    Bastion::block_until_stopped();
}

fn receiver_supervisor(supervisor: Supervisor) -> Supervisor {
    // We are doing the same as above
    supervisor.children(receiver_group)
}

fn receiver_group(children: Children) -> Children {
    // We create the second group of children
    children
        .with_redundancy(4)
        .with_recipient_target(RecipientTarget::named("Receiver"))
        // We create the function to exec when each children is called
        .with_exec(move |ctx: BastionContext| {
            async move {
                tracing::warn!("ready!");
                // We create a loop which run as long as the disptacher is alive
                loop {
                    msg! {
                        // We are waiting a msg
                        ctx.recv().await?,
                        // We define the behavior when we receive a new msg
                        raw_message: Arc<SignedMessage> => {
                            // We open the message
                            let message = Arc::try_unwrap(raw_message).unwrap();
                            msg! {
                                message,
                                _: _ => {
                                    tracing::error!("that was a broadcast!");
                                };
                            }
                        };
                        _: _ => {
                            tracing::error!("that wasn't a broadcast!")
                        };
                    }
                }
            }
        })
}
