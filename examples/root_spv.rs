use bastion::prelude::*;

fn main() {
    Bastion::platform();

    let message = String::from("a message");

    Bastion::spawn(
        |context: BastionContext, msg: Box<dyn Message>| {
            // Message can be used here.
            receive! { msg,
                String => |o| { println!("Received {}", o) },
                _ => println!("other message type...")
            }

            println!("spawned at root");

            // Rebind to the system
            context.hook();
        },
        message,
    );

    Bastion::start()
}
