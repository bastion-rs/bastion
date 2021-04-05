use bastion::prelude::*;
use bastion::{child_ref::Sent, distributor::*};
use tracing::Level;

#[tokio::main]
async fn main() {
    let subscriber = tracing_subscriber::fmt()
        .with_max_level(Level::INFO)
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

    let enthusiasts = Distributor::named("enthusiasts");
    // Staff -> send the actual schedule and misc infos to Attendees
    enthusiasts
        .tell_everyone(
            "Ok the actual schedule for the awesome conference is available, check it out!",
        )
        .expect("couldn't let everyone know the conference is available!");

    // let's make sure someone else receives the staff message
    // An attendee sends a thank you note to one staff member (and not bother everyone)

    // Create a new distribution target
    let staff = Distributor::named("staff");
    // Enthusiast -> Ask one of the staff members "when is the conference going to happen ?"
    if let Sent::Ask(answer) = staff
        .ask_one("when is the next conference going to happen?")
        .expect("couldn't ask question :(")
    {
        MessageHandler::new(answer.await.expect("woops")).on_tell(|reply: String, _sender_addr| {
            tracing::info!("received a reply to my message:\n{}", reply);
        });
    }

    staff
        .tell_one("the conference was amazing thank you so much!")
        .expect("couldn't thank the staff members :(");

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
                            tracing::info!("received a question: \n{}", message);
                            sender
                                .reply("uh i think it will be next month!".to_string())
                                .unwrap();
                        })
                        .on_tell(|message: Box<&str>, _| {
                            tracing::info!("received a message: \n{}", message);
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
                    MessageHandler::new(ctx.recv().await?).on_tell(
                        |message: std::sync::Arc<&str>, _| {
                            tracing::debug!(
                                "child {}, received a broadcast message:\n{}",
                                ctx.current().id(),
                                message
                            );
                        },
                    );
                }
            })
    })
}
