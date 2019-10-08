#[cfg(test)]
mod tests {
    use bastion::prelude::*;
    use log::LevelFilter;
    use std::{thread, time};

    fn awaiting(time: u64) {
        let ten_millis = time::Duration::from_millis(time);
        thread::sleep(ten_millis);
    }

    #[test]
    fn test_context_spawn() {
        let config = BastionConfig {
            log_level: LevelFilter::Debug,
            in_test: true,
        };
        Bastion::platform_from_config(config);

        let message = "Main Message".to_string();

        Bastion::supervisor("background-worker", "new-system")
            .strategy(SupervisionStrategy::OneForAll)
            .children(
                |p: BastionContext, _msg| {
                    // Children spawned from the body of supervisor.
                    println!("Started Child");
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

        awaiting(30);
    }
}
