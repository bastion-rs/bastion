use std::any::Any;

use bastion::children::RecipientTarget;
use bastion::distributor::*;
use bastion::{prelude::*, recipient::Recipient};
use tracing::Level;

fn main() {
    let subscriber = tracing_subscriber::fmt()
        .with_max_level(Level::WARN)
        .finish();
    tracing::subscriber::set_global_default(subscriber).unwrap();

    Bastion::init();

    Bastion::supervisor(supervised_staff).expect("Couldn't create supervisor chain.");

    Bastion::start();

    let staff = Distributor::named("staff");

    staff.ask_one("when is the conference going to happen?");

    Bastion::stop();
    Bastion::block_until_stopped();
}

fn supervised_staff(supervisor: Supervisor) -> Supervisor {
    supervisor.children(|children| 
        children
        .with_redundancy(10)
        .with_distributor(Distributor::named("staff"))
        .with_exec(move |ctx: BastionContext| async move {
            tracing::warn!("staff member ready!");
            loop {
                MessageHandler::new(ctx.recv().await?)
                    .on_question(|request: &str, _sender| {
                        tracing::warn!("received a question {}", request);
                    })
                    .on_tell(|message: &str, _| {
                        tracing::warn!("received a message {}", message);
                    })
                    .on_fallback(|unknown, sender_addr| {
                        tracing::warn!(
                            "uh oh, I received a message I didn't understand\n {:?}\nsender is\n{:?}",
                            unknown,
                            sender_addr
                        );
                    });
            }
        }))
}
