#[cfg(test)]
mod tests {
    use bastion::bastion::Bastion;
    
    use bastion::config::BastionConfig;
    use bastion::context::BastionContext;
    use bastion::supervisor::SupervisionStrategy;
    use log::LevelFilter;
    
    use std::sync::Once;
    use std::{fs, thread, time};
    
    

    static INIT: Once = Once::new();

    fn init() {
        INIT.call_once(|| {
            let config = BastionConfig {
                log_level: LevelFilter::Debug,
                in_test: true,
            };
            let _bastion = Bastion::platform_from_config(config);
        });
    }

    fn awaiting(time: u64) {
        let ten_millis = time::Duration::from_millis(time);
        thread::sleep(ten_millis);
    }

    #[test]
    fn spawn_at_root() {
        init();

        let message = "Kokojombo".to_string();
        let message2 = "Kokojombo Two".to_string();

        Bastion::spawn(
            |_p, _msg| {
                println!("root supervisor - spawn_at_root - 1");
            },
            message,
        );

        Bastion::spawn(
            |_p, _msg| {
                println!("root supervisor - spawn_at_root - 2");
            },
            message2,
        );

        Bastion::supervisor("k", "m");

        awaiting(10);
    }

    #[test]
    fn spawn_both_root_and_supervisor() {
        init();

        let message = "Kokojombo".to_string();
        let _message2 = "Kokojombo Two".to_string();

        Bastion::spawn(
            |_p, _msg| {
                println!("root supervisor - panic_roll_starting - 1");
                fs::read_to_string("cacophony").unwrap();
            },
            message,
        );

        awaiting(500);
    }

    #[test]
    fn spawn_with_supervisor_one_for_one() {
        init();

        let message = "Supervision Message".to_string();
        let message2 = "Some Other Message".to_string();

        Bastion::supervisor("background-worker", "new-system")
            .children(
                |_p, _msg| {
                    println!("new supervisor - panic_roll_starting - 1");
                    fs::read_to_string("cacophony").unwrap();
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
                        println!("CONTINUING {} :: {:?}", i, thread::current());
                        awaiting(200);
                        p.clone().hook();
                    }
                },
                message2,
                2_i32,
            )
            .launch();

        awaiting(500);
    }

    #[test]
    fn spawn_with_supervisor_rest_for_one() {
        init();

        let panicked_message = "Panicked Children Message".to_string();
        let stable_message = "Stable Children Message".to_string();

        Bastion::supervisor("background-worker", "new-system")
            .strategy(SupervisionStrategy::RestForOne)
            .children(
                |_p, _msg| {
                    println!("new supervisor - panic_process - 1");
                    fs::read_to_string("THERE_IS_NO_FILE_NAMED_THIS_AMIRITE").unwrap();
                },
                panicked_message,
                1_i32,
            )
            .children(
                |p: BastionContext, _msg| {
                    println!("new supervisor - stable_process - 1");
                    // No early exit
                    let mut i = 0;
                    loop {
                        i = i + 1;
                        println!("CONTINUING {} :: {:?}", i, thread::current());
                        awaiting(200);
                        p.clone().hook();
                    }
                },
                stable_message,
                1_i32,
            )
            .launch();

        awaiting(500);
    }

    #[test]
    fn spawn_with_supervisor_one_for_all() {
        init();

        let panicked_message = "Panicked Children Message".to_string();
        let stable_message = "Stable Children Message".to_string();

        Bastion::supervisor("background-worker", "new-system")
            .strategy(SupervisionStrategy::OneForAll)
            .children(
                |_p, _msg| {
                    println!("new supervisor - panic_process - 1");
                    fs::read_to_string("THERE_IS_NO_FILE_NAMED_THIS_AMIRITE").unwrap();
                },
                panicked_message,
                1_i32,
            )
            .children(
                |p: BastionContext, _msg| {
                    println!("new supervisor - stable_process - 1");
                    // No early exit
                    let mut i = 0;
                    loop {
                        i = i + 1;
                        println!("CONTINUING {} :: {:?}", i, thread::current());
                        awaiting(200);
                        p.clone().hook();
                    }
                },
                stable_message,
                1_i32,
            )
            .launch();

        awaiting(500);
    }
}
