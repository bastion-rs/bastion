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
    fn test_shutdown() {
        let config = BastionConfig {
            log_level: LevelFilter::Debug,
            in_test: true,
        };
        Bastion::platform_from_config(config);

        let message = "S1".to_string();
        let message2 = "S2".to_string();

        Bastion::spawn(
            |_p, _msg| {
                println!("root supervisor - spawn_at_root - 1");
            },
            message,
        );

        Bastion::supervisor("k", "m");

        awaiting(10);

        Bastion::spawn(
            |_p, _msg| {
                println!("root supervisor - spawn_at_root - 2");
            },
            message2,
        );

        println!("Shutdown initiated");

        Bastion::force_shutdown();
    }
}
