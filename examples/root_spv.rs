use bastion::bastion::Bastion;
use bastion::child::Message;
use bastion::context::BastionContext;

fn main() {
    Bastion::platform();

    let message = String::from("Some message to be passed");

    Bastion::spawn(
        |context: BastionContext, msg: Box<dyn Message>| {
            // Message can be casted and reused here.
            let received_msg = msg.as_any().downcast_ref::<String>().unwrap();

            println!("Received message: {:?}", received_msg);
            println!("root supervisor - spawn_at_root - 1");

            // Rebind to the system
            context.hook();
        },
        message,
    );

    Bastion::start()
}
