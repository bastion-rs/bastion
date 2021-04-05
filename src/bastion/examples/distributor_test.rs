use bastion::distributor::*;
use bastion::prelude::*;
use tracing::Level;

fn main() {
    let subscriber = tracing_subscriber::fmt()
        .with_max_level(Level::WARN)
        .finish();
    tracing::subscriber::set_global_default(subscriber).unwrap();

    Bastion::init();

    Bastion::supervisor(supervised_staff).unwrap();

    Bastion::start();

    std::thread::sleep(std::time::Duration::from_secs(1));
    let staff = Distributor::named("staff");

    staff.tell_one(42_u8);

    Bastion::stop();
    Bastion::block_until_stopped();
}

fn supervised_staff(supervisor: Supervisor) -> Supervisor {
    supervisor.children(|children| {
        children
            .with_redundancy(10)
            .with_distributor(Distributor::named("staff"))
            .with_exec(move |ctx: BastionContext| async move {
                tracing::warn!("staff member ready!");
                loop {
                    MessageHandler::new(ctx.recv().await?)
                    .on_tell(|message: Box<u8>, _| {
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
            })
    })
}
