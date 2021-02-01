use anyhow::Result as AnyResult;
use bastion::prelude::*;
#[cfg(feature = "tokio-runtime")]
use tokio;
use tracing::{error, warn, Level};

/// `cargo run --features=tokio-runtime --example hello_tokio`
///
/// We are focusing on the contents of the msg! macro here.
/// If you would like to understand how the rest works,
/// Have a look at the `hello_world.rs` example instead :)
///
/// Log output:
///
/// Jan 31 14:55:55.677  WARN hello_tokio: just spawned!
/// Jan 31 14:55:56.678  WARN hello_tokio: Ok let's handle a message now.
/// Jan 31 14:55:56.678 ERROR hello_tokio: just received hello, world!
/// Jan 31 14:55:56.678  WARN hello_tokio: sleeping for 2 seconds without using the bastion executor
/// Jan 31 14:55:58.680  WARN hello_tokio: and done!
/// Jan 31 14:55:58.681  WARN hello_tokio: let's sleep for 5 seconds within a blocking block
/// Jan 31 14:56:03.682  WARN hello_tokio: awaited 5 seconds
/// Jan 31 14:56:03.682 ERROR hello_tokio: waited for the blocking! to be complete!
/// Jan 31 14:56:03.682 ERROR hello_tokio: not waiting for spawn! to be complete, moving on!
/// Jan 31 14:56:03.683  WARN hello_tokio: let's sleep for 10 seconds within a spawn block
/// Jan 31 14:56:13.683  WARN hello_tokio: the spawn! is complete
/// Jan 31 14:56:15.679  WARN hello_tokio: we're done, stopping the bastion!
#[cfg(feature = "tokio-runtime")]
#[tokio::main]
async fn main() -> AnyResult<()> {
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
                warn!("just spawned!");
                tokio::time::sleep(std::time::Duration::from_secs(1)).await;
                warn!("Ok let's handle a message now.");
                msg! {
                    ctx.recv().await?,
                    msg: &'static str => {
                        // Printing the incoming msg
                        error!("just received {}", msg);

                        warn!("sleeping for 2 seconds without using the bastion executor");
                        tokio::time::sleep(std::time::Duration::from_secs(2)).await;
                        warn!("and done!");

                        // let's wait until a tokio powered future is complete
                        run!(blocking! {
                            warn!("let's sleep for 5 seconds within a blocking block");
                            tokio::time::sleep(std::time::Duration::from_secs(5)).await;
                            warn!("awaited 5 seconds");
                        });
                        error!("waited for the blocking! to be complete!");

                        // let's spawn a tokio powered future and move on
                        spawn! {
                            warn!("let's sleep for 10 seconds within a spawn block");
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

    warn!("we're done, asking bastion to stop!");
    // We are done, stopping the bastion!
    Bastion::stop();
    warn!("bastion stopped!");
    Ok(())
}

#[cfg(not(feature = "tokio-runtime"))]
fn main() {
    panic!("this example requires the tokio-runtime feature: `cargo run --features=tokio-runtime --example hello_tokio`")
}
