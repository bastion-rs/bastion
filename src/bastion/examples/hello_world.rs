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
    let workers = Bastion::children(|children| {
        // We are creating the function to exec
        children.with_exec(|ctx: BastionContext| {
            async move {
                // We are defining a behavior when a msg is received
                msg! {
                    // We are waiting a msg
                    ctx.recv().await?,
                    // We are catching a msg
                    msg: &'static str =!> {
                        // Printing the incoming msg
                        println!("{}", msg);
                        // Sending the asnwer
                        answer!(ctx, msg).expect("couldn't reply :(");
                    };
                    _: _ => ();
                }
                Ok(())
            }
        })
    })
    .expect("Couldn't create the children group.");
    // We are creating the asker
    let asker = async move {
        // We are getting the first (and only) worker
        let answer = workers.elems()[0]
            .ask_anonymously("hello, world!")
            .expect("Couldn't send the message.");
        // We are waiting for the asnwer
        answer.await.expect("couldn't receive answer");
    };
    // We are running the asker in the current blocked thread
    run!(asker);
    // We are stopping bastion here
    Bastion::stop();
}
