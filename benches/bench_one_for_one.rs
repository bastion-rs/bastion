#![feature(test)]

extern crate test;

#[cfg(test)]
mod tests {
    use super::*;
    use bastion::bastion::Bastion;
    use bastion::bastion::PLATFORM;
    use bastion::config::BastionConfig;
    use bastion::context::BastionContext;
    use bastion::supervisor::SupervisionStrategy;
    use log::LevelFilter;
    use std::borrow::{Borrow, BorrowMut};
    use std::sync::Once;
    use std::{fs, thread, time};
    use test::Bencher;
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

    #[bench]
    fn spawn_with_supervisor_one_for_one(b: &mut Bencher) {
        fn closure() {
            init();

            let message = "Supervision Message".to_string();

            Bastion::spawn(
                |p, msg| {
                    panic!("root supervisor - spawn_at_root - 1");
                },
                message,
            );

            awaiting(100);
        }

        b.iter(|| closure());
    }
}
