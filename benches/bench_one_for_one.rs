#![feature(test)]

extern crate test;

#[cfg(test)]
mod tests {
    use super::*;
    use bastion::bastion::Bastion;
    
    use bastion::config::BastionConfig;
    
    
    use log::LevelFilter;
    
    use std::sync::Once;
    use std::{thread, time};
    use test::Bencher;
    
    

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

    #[bench]
    fn spawn_with_supervisor_one_for_one(b: &mut Bencher) {
        fn closure() {
            init();

            let message = "Supervision Message".to_string();

            Bastion::spawn(
                |_p, _msg| {
                    panic!("root supervisor - spawn_at_root - 1");
                },
                message,
            );

            awaiting(100);
        }

        b.iter(|| closure());
    }
}
