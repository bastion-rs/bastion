///! Create a conference.
///!
///! 1st Group
///! Staff (5) - Going to organize the event // OK
///!
///! 2nd Group
///! Enthusiasts (50) - interested in participating to the conference (haven't registered yet) // OK
///!
///! 3rd Group
///! Attendees (empty for now) - Participate
///!
///! Enthusiast -> Ask one of the staff members "when is the conference going to happen ?" // OK
///! Broadcast / Question => Answer 0 or 1 Staff members are going to reply eventually? // OK
///!
///! Staff -> Send a Leaflet to all of the enthusiasts, letting them know that they can register. // OK
///!
///! "hey conference <awesomeconference> is going to happen. will you be there?"
///! Broadcast / Question -> if people reply with YES => fill the 3rd group
///! some enthusiasts are now attendees
///!
///! Staff -> send the actual schedule and misc infos to Attendees
///! Broadcast / Statement (Attendees)
///!
///! An attendee sends a thank you note to one staff member (and not bother everyone)
///! One message / Statement (Staff) // OK
///!
///! ```rust
///!     let staff = Distributor::named("staff");
///!     let enthusiasts = Distributor::named("enthusiasts");
///!     let attendees = Disitributor::named("attendees");
///!     // Enthusiast -> Ask the whole staff "when is the conference going to happen ?"
///!     ask_one(Message + Clone) -> Result<impl Future<Output = Reply>, CouldNotSendError>
///!     // await_one // await_all
///!     // first ? means "have we been able to send the question?"
///!     // it s in a month
///!     let replies = staff.ask_one("when is the conference going to happen ?")?.await?;
///!     ask_everyone(Message + Clone) -> Result<impl Stream<Item = Reply>, CouldNotSendError>
///!     let participants = enthusiasts.ask_everyone("here's our super nice conference, it s happening people!").await?;
///!     for participant in participants {
///!         // grab the sender and add it to the attendee recipient group
///!     }
///!     // send the schedule
///!     tell_everyone(Message + Clone) -> Result<(), CouldNotSendError>
///!     attendees.tell_everyone("hey there, conf is in a week, here s where and how it s going to happen")?;
///!     // send a thank you note
///!     tell(Message) -> Result<(), CouldNotSendError>
///!     staff.tell_one("thank's it was amazing")?;
///!     children
///!         .with_redundancy(10)
///!         .with_distributor(Distributor::named("staff"))
///!         // We create the function to exec when each children is called
///!         .with_exec(move |ctx: BastionContext| async move { /* ... */ })
///!     children
///!         .with_redundancy(100)
///!         .with_distributor(Distributor::named("enthusiasts"))
///!         // We create the function to exec when each children is called
///!         .with_exec(move |ctx: BastionContext| async move { /* ... */ })
///!     children
///!         .with_redundancy(0)
///!         .with_distributor(Distributor::named("attendees"))
///!         // We create the function to exec when each children is called
///!         .with_exec(move |ctx: BastionContext| async move { /* ... */ })
///! ```


/*
* cargo.toml:
*
* [dependencies]
* anyhow = "1.0.40"
* bastion = { git = "https://github.com/bastion-rs/bastion" }
* rand = "0.8.3"
* tracing = "0.1.25"
* tracing-subscriber = "0.2.17"
*
*/

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

/// cargo r --features=tokio-runtime --example distributor
#[cfg(feature = "tokio-runtime")]
#[tokio::main]
async fn main() -> AnyResult<()> {
    run()
}

#[cfg(not(feature = "tokio-runtime"))]
fn main() -> AnyResult<()> {
    run()
}

fn run() -> AnyResult<()> {
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
    sleep(std::time::Duration::from_secs(5));

    let staff = Distributor::named("staff");
    let enthusiasts = Distributor::named("enthusiasts");
    let attendees = Distributor::named("attendees");

    // Enthusiast -> Ask one of the staff members "when is the conference going to happen ?"
    let reply: Result<String, SendError> = run!(async {
        staff
            .request("when is the next conference going to happen?")
            .await
            .expect("couldn't receive reply")
    });

    tracing::error!("{:?}", reply); // Ok("Next month!")

    // "hey conference <awesomeconference> is going to happen. will you be there?"
    // Broadcast / Question -> if people reply with YES => fill the 3rd group
    let answers = enthusiasts
        .ask_everyone("hey, the conference is going to happen, will you be there?")
        .expect("couldn't ask everyone");

    for answer in answers.into_iter() {
        run!(async move {
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

    // Let's wait until the conference is over 8D
    sleep(std::time::Duration::from_secs(5));

    // An attendee sends a thank you note to one staff member (and not bother everyone)
    staff
        .tell_one("the conference was amazing thank you so much!")
        .context("couldn't thank the staff members :(")?;

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
                sender.reply("Next month!".to_string()).unwrap();
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

#[cfg(feature = "tokio-runtime")]
fn sleep(duration: std::time::Duration) {
    run!(async {
        tokio::time::sleep(duration).await;
    });
}

#[cfg(not(feature = "tokio-runtime"))]
fn sleep(duration: std::time::Duration) {
    std::thread::sleep(duration);
}
