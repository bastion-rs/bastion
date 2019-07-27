use bastion::prelude::*;
use std::{fs, thread};

fn main() {
    Bastion::platform();

    let message = "Supervision Message".to_string();
    let message2 = "Some Other Message".to_string();

    // Name of the supervisor, and system of the new supervisor
    // By default if you don't specify Supervisors use "One for One".
    // We are going to take a look at "One For All" strategy.
    Bastion::supervisor("background-worker", "new-system")
        .strategy(SupervisionStrategy::OneForAll)
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
                    // Start everyone under this supervisor. Immediately for all of them.
                    println!("Going to fail  {} :: {:?}", i, thread::current());

                    // Hook to rebind to the system.
                    p.clone().hook();
                }
            },
            message2,
            2_i32,
        )
        .launch();

    Bastion::start()
}
