use bastion::prelude::*;
use log::LevelFilter;

fn main() {
    let config = BastionConfig {
        log_level: LevelFilter::Debug,
        in_test: false,
    };

    Bastion::platform_from_config(config);

    let message = "Main Message".to_string();

    // Name of the supervisor, and system of the new supervisor
    // By default if you don't specify Supervisors use "One for One".
    // We are going to take a look at "One For All" strategy.
    Bastion::supervisor("background-worker", "new-system")
        .strategy(SupervisionStrategy::OneForAll)
        .children(
            |p: BastionContext, _msg| {
                // Children spawned from the body of supervisor.
                println!("Started Child");

                // Spawn child from the child context.
                // All rules apply at the supervisor level also to here.
                // Supervisor -> Child -> Child
                //                  \
                //                   ---> Child
                for ctx_message in 1..=2 {
                    p.clone().spawn(
                        |sub_p: BastionContext, sub_msg: Box<dyn Message>| {
                            receive! { sub_msg,
                                i32 => |msg| {
                                    if msg == 1 {
                                        println!("First one");
                                        panic!("Panic over the first one");
                                    } else {
                                        println!("Second one");
                                    }
                                },
                                _ => println!("Message not known")
                            }

                            // Use blocking hook to commence restart of children
                            // that has finished their jobs.
                            sub_p.blocking_hook();
                        },
                        ctx_message,
                        1,
                    );
                }

                // Hook to rebind to the system.
                p.hook();
            },
            message,
            1_i32,
        )
        .launch();

    Bastion::start()
}
