use bastion::distributor::*;
use bastion::prelude::*;
use tracing::Level;

fn main() {
    let subscriber = tracing_subscriber::fmt()
        .with_max_level(Level::WARN)
        .finish();
    tracing::subscriber::set_global_default(subscriber).unwrap();

    // Initialize bastion
    Bastion::init();

    Bastion::supervisor(supervised_staff)
        .and_then(|_| Bastion::supervisor(supervised_enthusiasts))
        .unwrap();

    Bastion::start();

    // Wait a bit until everyone is ready
    std::thread::sleep(std::time::Duration::from_secs(1));

    // Create a new distribution target
    let staff = Distributor::named("staff");
    // Enthusiast -> Ask one of the staff members "when is the conference going to happen ?"
    staff.ask_one("when is the next conference going to happen?");

    let enthusiasts = Distributor::named("enthusiasts");
    // Staff -> send the actual schedule and misc infos to Attendees
    enthusiasts.tell_everyone(
        "Ok the actual schedule for the awesome conference is available, check it out!",
    );

    // let's make sure someone else receives the staff message
    // An attendee sends a thank you note to one staff member (and not bother everyone)
    staff.tell_one("the conference was amazing thank you so much!");

    Bastion::stop();
    Bastion::block_until_stopped();
}

fn supervised_staff(supervisor: Supervisor) -> Supervisor {
    supervisor.children(|children| {
        children
            .with_redundancy(10)
            .with_distributor(Distributor::named("staff"))
            .with_exec(move |ctx: BastionContext| async move {
                loop {
                    MessageHandler::new(ctx.recv().await?)
                    .on_question(|message: Box<&str>, sender| {
                        tracing::warn!("received a question: \n{}", message);
                        // TODO: FIGURE THIS OUT SOMEDAY
                        // sender.reply("uh i think it will be next month!").unwrap();
                    })
                    .on_tell(|message: Box<&str>, _| {
                        tracing::warn!("received a message: \n{}", message);
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

fn supervised_enthusiasts(supervisor: Supervisor) -> Supervisor {
    supervisor.children(|children| {
        children
            .with_redundancy(100)
            .with_distributor(Distributor::named("enthusiasts"))
            .with_exec(move |ctx: BastionContext| async move {
                loop {
                    MessageHandler::new(ctx.recv().await?)
                    .on_tell(|message: std::sync::Arc<&str>, _| {
                        tracing::warn!("child {}, received a broadcast message:\n{}",ctx.current().id(), message);
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
