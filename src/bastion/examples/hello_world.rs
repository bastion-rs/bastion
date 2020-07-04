use bastion::prelude::*;

///
/// Prologue:
/// The most classic of all examples and especially the essential hello, world!
///
fn main() {
    // We need bastion to run our program
    Bastion::init();
    // We are starting the Bastion program now
    Bastion::start();
    // We are creating the group of children which will received the message
    let children = Bastion::children(|children| {
        // We are creating the function to exec
        children.with_exec(|ctx: BastionContext| {
            async move {
                msg! {
                  // We are waiting a msg
                  ctx.recv().await?,
                  ref msg: &'static str => {
                    // We are logging the msg broadcasted bellow
                    println!("{}", msg);
                  };
                  _: _ => ();
                }
                // We are stopping bastion here
                Bastion::stop();
                Ok(())
            }
        })
    })
    .expect("Couldn't create the children group.");
    // We are creating the message to broadcast to the children group
    children
        .broadcast("Hello, world!")
        .expect("Couldn't broadcast the message.");
    // We are waiting until the Bastion has stopped or got killed
    Bastion::block_until_stopped();
}
