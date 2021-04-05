use anyhow::{anyhow, Context, Result as AnyResult};
use bastion::distributor::*;
use bastion::prelude::*;
use tracing::Level;

// true if the person attends the conference
#[derive(Debug)]
struct RSVP {
    attends: bool,
    child_ref: ChildRef,
}

#[derive(Debug, Clone)]
struct ConferenceSchedule {
    start: std::time::Duration,
    end: std::time::Duration,
    misc: String,
}

#[tokio::main]
async fn main() -> AnyResult<()> {
    let subscriber = tracing_subscriber::fmt()
        .with_max_level(Level::INFO)
        .finish();
    tracing::subscriber::set_global_default(subscriber).unwrap();

    // Initialize bastion
    Bastion::init();

    // 1st Group
    Bastion::supervisor(|supervisor| {
        supervisor.children(|children| {
            // Iniit staff
            // Staff (5 members) - Going to organize the event
            children
                .with_redundancy(5)
                .with_distributor(Distributor::named("staff"))
                .with_exec(organize_the_event)
        })
    })
    // 2nd Group
    .and_then(|_| {
        Bastion::supervisor(|supervisor| {
            supervisor.children(|children| {
                // Enthusiasts (50) - interested in participating to the conference (haven't registered yet)
                children
                    .with_redundancy(50)
                    .with_distributor(Distributor::named("enthusiasts"))
                    .with_exec(be_interested_in_the_conference)
            })
        })
    })
    .map_err(|_| anyhow!("couldn't setup the bastion"))?;

    Bastion::start();

    // Wait a bit until everyone is ready
    std::thread::sleep(std::time::Duration::from_secs(1));

    let staff = Distributor::named("staff");
    let enthusiasts = Distributor::named("enthusiasts");
    let attendees = Distributor::named("attendees");

    // Enthusiast -> Ask one of the staff members "when is the conference going to happen ?"
    let answer = staff.ask_one("when is the next conference going to happen?")?;
    MessageHandler::new(
        answer
            .await
            .expect("coulnd't find out when the next conference is going to happen :("),
    )
    .on_tell(|reply: String, _sender_addr| {
        tracing::info!("received a reply to my message:\n{}", reply);
    });

    // "hey conference <awesomeconference> is going to happen. will you be there?"
    // Broadcast / Question -> if people reply with YES => fill the 3rd group
    let answers = enthusiasts
        .ask_everyone("hey, the conference is going to happen, will you be there?")
        .expect("couldn't ask everyone");

    for answer in answers.into_iter() {
        MessageHandler::new(answer.await.expect("couldn't receive reply"))
            .on_tell(|rsvp: RSVP, _| {
                if rsvp.attends {
                    tracing::info!("{:?} will be there! :)", rsvp.child_ref.id());
                    attendees
                        .subscribe(rsvp.child_ref)
                        .expect("couldn't subscribe attendee");
                } else {
                    tracing::error!("{:?} won't make it :(", rsvp.child_ref.id());
                }
            })
            .on_fallback(|unknown, _sender_addr| {
                tracing::error!(
                    "distributor_test: uh oh, I received a message I didn't understand\n {:?}",
                    unknown
                );
            });
    }

    // Ok now that attendees have subscribed, let's send information around!
    tracing::info!("Let's send invitations!");
    // Staff -> send the actual schedule and misc infos to Attendees
    let total_sent = attendees
        .tell_everyone(ConferenceSchedule {
            start: std::time::Duration::from_secs(60),
            end: std::time::Duration::from_secs(3600),
            misc: "it's going to be amazing!".to_string(),
        })
        .context("couldn't let everyone know the conference is available!")?;

    tracing::error!("total number of attendees: {}", total_sent.len());

    tracing::info!("the conference is running!");
    tokio::time::sleep(std::time::Duration::from_secs(10)).await;

    // An attendee sends a thank you note to one staff member (and not bother everyone)
    staff
        .tell_one("the conference was amazing thank you so much!")
        .context("couldn't thank the staff members :(")?;

    tokio::time::sleep(std::time::Duration::from_secs(1)).await;
    // And we're done!
    Bastion::stop();

    // BEWARE, this example doesn't return
    Bastion::block_until_stopped();

    Ok(())
}

async fn organize_the_event(ctx: BastionContext) -> Result<(), ()> {
    loop {
        MessageHandler::new(ctx.recv().await?)
            .on_question(|message: &str, sender| {
                tracing::info!("received a question: \n{}", message);
                sender
                    .reply("uh i think it will be next month!".to_string())
                    .unwrap();
            })
            .on_tell(|message: &str, _| {
                tracing::info!("received a message: \n{}", message);
            })
            .on_fallback(|unknown, _sender_addr| {
                tracing::error!(
                    "staff: uh oh, I received a message I didn't understand\n {:?}",
                    unknown
                );
            });
    }
}

async fn be_interested_in_the_conference(ctx: BastionContext) -> Result<(), ()> {
    loop {
        MessageHandler::new(ctx.recv().await?)
            .on_tell(|message: std::sync::Arc<&str>, _| {
                tracing::info!(
                    "child {}, received a broadcast message:\n{}",
                    ctx.current().id(),
                    message
                );
            })
            .on_tell(|schedule: ConferenceSchedule, _| {
                tracing::info!(
                    "child {}, received broadcast conference schedule!:\n{:?}",
                    ctx.current().id(),
                    schedule
                );
            })
            .on_question(|message: &str, sender| {
                tracing::info!("received a question: \n{}", message);
                // ILL BE THERE!
                sender
                    .reply(RSVP {
                        attends: rand::random(),
                        child_ref: ctx.current().clone(),
                    })
                    .unwrap();
            });
    }
}
