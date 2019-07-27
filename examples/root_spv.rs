use bastion::prelude::*;

fn main() {
    Bastion::platform();

    let message = String::from("Some message to be passed");

    Bastion::spawn(
        |context: BastionContext, msg: Box<dyn Message>| {
            // Message can be used here.
            match Receive::<String>::from(msg) {
                Receive(Some(o)) => println!("Received {}", o),
                _ => println!("other message type...")
            }

            println!("root supervisor - spawn_at_root - 1");

            // Rebind to the system
            context.hook();
        },
        message,
    );

    Bastion::start()
}
