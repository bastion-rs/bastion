use bastion::{message::MessageHandler, prelude::*};

use tracing::{error, info};

// This terribly slow implementation
// will allow us to be rough on the cpu
fn fib(n: usize) -> usize {
    if n == 0 || n == 1 {
        n
    } else {
        fib(n - 1) + fib(n - 2)
    }
}

// This terrible helper is converting `fib 50` into a tuple ("fib", 50)
// we might want to use actual serializable / deserializable structures
// in the real world
fn deserialize_into_fib_command(message: String) -> (String, usize) {
    let arguments: Vec<&str> = message.split(' ').collect();
    let command = arguments.first().map(|s| s.to_string()).unwrap_or_default();
    let number = usize::from_str_radix(arguments.get(1).unwrap_or(&"0"), 10).unwrap_or(0);
    (command, number)
}

// This is the heavylifting.
// A child will wait for a message, and try to process it.
async fn fib_child_task(ctx: BastionContext) -> Result<(), ()> {
    loop {
        MessageHandler::new(ctx.recv().await?)
            .on_question(|request: String, sender| {
                let (command, number) = deserialize_into_fib_command(request);
                if command == "fib" {
                    sender
                        .reply(format!("{}", fib(number)))
                        .expect("couldn't reply :(");
                } else {
                    sender
                        .reply(format!(
                            "I'm sorry I didn't understand the task I was supposed to do"
                        ))
                        .expect("couldn't reply :(");
                }
            })
            .on_broadcast(|broadcast: &String, _sender_addr| {
                info!("received broadcast: {:?}", *broadcast);
            })
            .on_tell(|message: String, _sender_addr| {
                info!("someone told me something: {}", message);
            })
            .on_fallback(|unknown, _sender_addr| {
                error!(
                    "uh oh, I received a message I didn't understand\n {:?}",
                    unknown
                );
            });
    }
}

// This little helper allows me to send a request, and get a reply.
// The types are `String` for this quick example, but there's a way for us to do better.
// We will see this in other examples.
async fn request(child: &ChildRef, body: String) -> std::io::Result<String> {
    let answer = child
        .ask_anonymously(body)
        .expect("couldn't perform request");

    Ok(
        MessageHandler::new(answer.await.expect("couldn't receive answer"))
            .on_tell(|reply, _sender_addr| reply)
            .on_fallback(|unknown, _sender_addr| {
                error!(
                    "uh oh, I received a message I didn't understand: {:?}",
                    unknown
                );
                "".to_string()
            }),
    )
}

// RUST_LOG=info cargo run --example fibonacci_message_handler
fn main() {
    // This will allow us to have nice colored logs when we run the program
    env_logger::init();

    // We need a bastion in order to run everything
    Bastion::init();
    Bastion::start();

    // Spawn 4 children that will execute our fibonacci task
    let children =
        Bastion::children(|children| children.with_redundancy(4).with_exec(fib_child_task))
            .expect("couldn't create children");

    // Broadcasting 1 message to the children
    // Have a look at the console output
    // to see 1 log entry for each child!
    children
        .broadcast("Hello there :)".to_string())
        .expect("Couldn't broadcast to the children.");

    let mut fib_to_compute = 35;
    for child in children.elems() {
        child
            .tell_anonymously("shhh here's a message, don't tell anyone.".to_string())
            .expect("Couldn't whisper to child.");

        let now = std::time::Instant::now();
        // by using run!, we are blocking.
        // we could have used spawn! instead,
        // to run everything in parallel.
        let fib_reply = run!(request(child, format!("fib {}", fib_to_compute)))
            .expect("send_command_to_child failed");

        println!(
            "fib({}) = {} - Computed in {}ms",
            fib_to_compute,
            fib_reply,
            now.elapsed().as_millis()
        );
        // Let's not go too far with the fib sequence
        // Otherwise the computer may take a while!
        fib_to_compute += 2;
    }
    Bastion::stop();
    Bastion::block_until_stopped();
}

// Compiling bastion v0.3.5-alpha (/home/ignition/Projects/oss/bastion/src/bastion)
// Finished dev [unoptimized + debuginfo] target(s) in 1.07s
//  Running `target/debug/examples/fibonacci`
// [2020-05-08T14:00:53Z INFO  bastion::system] System: Initializing.
// [2020-05-08T14:00:53Z INFO  bastion::system] System: Launched.
// [2020-05-08T14:00:53Z INFO  bastion::system] System: Starting.
// [2020-05-08T14:00:53Z INFO  bastion::system] System: Launching Supervisor(00000000-0000-0000-0000-000000000000).
// [2020-05-08T14:00:53Z INFO  fibonacci] someone told me something: shhh here's a message, don't tell anyone.
// [2020-05-08T14:00:53Z INFO  fibonacci] received broadcast: "Hello there :)"
// [2020-05-08T14:00:53Z INFO  fibonacci] received broadcast: "Hello there :)"
// [2020-05-08T14:00:53Z INFO  fibonacci] received broadcast: "Hello there :)"
// fib(35) = 9227465 - Computed in 78ms
// [2020-05-08T14:00:53Z INFO  fibonacci] received broadcast: "Hello there :)"
// [2020-05-08T14:00:53Z INFO  fibonacci] someone told me something: shhh here's a message, don't tell anyone.
// fib(37) = 24157817 - Computed in 196ms
// [2020-05-08T14:00:53Z INFO  fibonacci] someone told me something: shhh here's a message, don't tell anyone.
// fib(39) = 63245986 - Computed in 512ms
// [2020-05-08T14:00:54Z INFO  fibonacci] someone told me something: shhh here's a message, don't tell anyone.
// fib(41) = 165580141 - Computed in 1327ms
// [2020-05-08T14:00:55Z INFO  bastion::system] System: Stopping.
