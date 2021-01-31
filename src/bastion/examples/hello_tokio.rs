use bastion::prelude::*;
#[cfg(feature = "runtime-tokio")]
use tokio;
use tracing::{error, info, warn, Level};

/// `cargo run --features=runtime-tokio --example hello_tokio`
///
/// We are focusing on the contents of the msg! macro here.
/// If you would like to understand how the rest works,
/// Have a look at the `hello_world.rs` example instead :)
///
/// Log output:
///
/// Jan 31 11:54:14.372 ERROR hello_tokio: just received hello, world!
/// Jan 31 11:54:14.472  WARN hello_tokio: awaited 5 seconds
/// Jan 31 11:54:14.472 ERROR hello_tokio: waited for the blocking! to be complete!
/// Jan 31 11:54:14.473 ERROR hello_tokio: not waiting for spawn! to be complete, moving on!
/// Jan 31 11:54:24.473  WARN hello_tokio: the spawn! is complete
/// Jan 31 11:54:34.372  WARN hello_tokio: we're done, stopping the bastion!
#[cfg(feature = "runtime-tokio")]
fn main() {
    // Initialize tracing logger
    // so we get nice output on the console.
    let subscriber = tracing_subscriber::fmt()
        .with_max_level(Level::WARN)
        .finish();
    tracing::subscriber::set_global_default(subscriber).unwrap();

    Bastion::init();
    Bastion::start();
    let workers = Bastion::children(|children| {
        children.with_exec(|ctx: BastionContext| {
            async move {
                msg! {
                    ctx.recv().await?,
                    msg: &'static str => {
                        // Printing the incoming msg
                        error!("just received {}", msg);

                        // let's wait until a tokio powered future is complete
                        run!(blocking! {
                            tokio::time::sleep(std::time::Duration::from_secs(5)).await;
                            warn!("awaited 5 seconds");
                        });
                        error!("waited for the blocking! to be complete!");

                        // let's spawn a tokio powered future and move on
                        spawn! {
                            tokio::time::sleep(std::time::Duration::from_secs(10)).await;
                            warn!("the spawn! is complete");
                        };
                        error!("not waiting for spawn! to be complete, moving on!");
                    };
                    _: _ => ();
                }
                Ok(())
            }
        })
    })
    .expect("Couldn't create the children group.");

    let asker = async {
        workers.elems()[0]
            .tell_anonymously("hello, world!")
            .expect("Couldn't send the message.");
    };
    run!(asker);

    // Let's wait until the blocking! and the spawn! are complete on the child side.
    run!(blocking!({
        std::thread::sleep(std::time::Duration::from_secs(20))
    }));

    warn!("we're done, stopping the bastion!");

    // We are done, stopping the bastion!
    Bastion::stop();
}

#[cfg(not(feature = "runtime-tokio"))]
fn main() {
    panic!("this example requires the runtime-tokio feature: `cargo run --features=runtime-tokio --example hello_tokio`")
}
