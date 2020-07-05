use bastion::prelude::*;

///
/// Prologue:
/// The most classic of all examples and especially the essential hello, world!
///
fn main() {
    // We need bastion to run our program
    Bastion::init();
    Bastion::start();
    // We are creating a group of children
    Bastion::children(|children| {
        // We are creating the function to exec
        children.with_exec(|ctx: BastionContext| {
            async move {
                // We are creating the asker
                let answer = ctx
                    // It sending a message to the current child and return the Answer
                    .ask(&ctx.current().addr(), "hello, world!")
                    .expect("Couldn't send the message.");
                // We are defining a behavior when a msg is received
                msg! {
                  // We are waiting a msg
                  ctx.recv().await?,
                  // We are catching a msg
                  msg: &'static str =!> {
                    // We are answering to a msg
                    let _ = answer!(ctx, msg);
                  };
                  _: _ => ();
                }
                // We are waiting the answer
                let (msg, _) = answer.await?.extract();
                // We are casting the received message into it waiting type
                let msg: &str = msg.downcast().unwrap();
                // We are printing the message
                println!("{}", msg);
                // We are stopping bastion here
                Bastion::stop();
                Ok(())
            }
        })
    })
    .expect("Couldn't create the children group.");
    // We are waiting until the Bastion has stopped or got killed
    Bastion::block_until_stopped();
}
