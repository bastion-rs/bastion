#[cfg(test)]
mod tests {
    use bastion::prelude::*;

    use log::LevelFilter;
    use std::borrow::{Borrow, BorrowMut};
    use std::sync::Once;
    use std::{fs, thread, time};
    use tokio::prelude::*;
    use tokio::runtime::{Builder, Runtime};

    static INIT: Once = Once::new();

    fn init() {
        INIT.call_once(|| {
            let config = BastionConfig {
                log_level: LevelFilter::Debug,
                in_test: true,
            };
            let bastion = Bastion::platform_from_config(config);
        });
    }

    fn awaiting(time: u64) {
        let ten_millis = time::Duration::from_millis(time);
        thread::sleep(ten_millis);
    }

    #[test]
    fn spawn_test_with_prelude() {
        init();

        let message = "Kokojombo".to_string();
        let message2 = "Kokojombo Two".to_string();

        Bastion::spawn(
            |p, msg| {
                println!("root supervisor - spawn_at_root - 1");
            },
            message,
        );

        Bastion::spawn(
            |p, msg| {
                println!("root supervisor - spawn_at_root - 2");
            },
            message2,
        );

        Bastion::supervisor("k", "m");

        awaiting(10);
    }
}
