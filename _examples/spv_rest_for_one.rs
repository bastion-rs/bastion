use bastion::prelude::*;
use core::time;
use std::{fs, thread};

fn main() {
    Bastion::platform();

    let message = "Supervision Message".to_string();
    let message2 = "Some Other Message".to_string();

    // Name of the supervisor, and system of the new supervisor
    // By default if you don't specify Supervisors use "One for One".
    // We are going to take a look at "Rest for One" strategy.
    Bastion::supervisor("background-worker", "new-system")
        .strategy(SupervisionStrategy::RestForOne)
        .children(
            |p: BastionContext, _msg| {
                println!("File below doesn't exist so it will panic.");
                fs::read_to_string("cacophony").unwrap();

                // Hook to rebind to the system.
                p.hook();
            },
            message,
            1_i32,
        )
        .children(
            |p: BastionContext, _msg| {
                // No early exit
                let mut i = 0;
                loop {
                    i = i + 1;
                    // Going to fail with other children in this group.
                    // Difference between "Rest for One" and "One for All" is:
                    // * One for All sends PoisonPill to everyone.
                    // * Rest for One sends only PoisonPill to rest other than related group.
                    println!("Going to fail {} :: {:?}", i, thread::current());

                    // Behave like some heavy-lifting work occuring.
                    let ten_millis = time::Duration::from_millis(2000);
                    thread::sleep(ten_millis);

                    // Hook to rebind to the system.
                    p.clone().hook();
                }
            },
            message2, // Message which will be passed around.
            1_i32,    // How many child will be replicated from the worker closure.
        )
        .launch();

    Bastion::start()
}
