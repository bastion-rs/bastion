use bastion::prelude::*;
use proptest::prelude::*;
use std::sync::Once;

static START: Once = Once::new();

proptest! {
    #![proptest_config(ProptestConfig::with_cases(1_000))]
    #[test]
    fn proptest_redundancy(r in std::usize::MIN..32) {
        START.call_once(|| {
            Bastion::init();
        });
        Bastion::start();

        Bastion::children(|children| {
            children
                // shrink over the redundancy
                .with_redundancy(r)
                .with_exec(|_ctx: BastionContext| {
                    async move {
                        loop {
                            std::mem::drop(());
                        }
                    }
                })
        }).expect("Coudn't spawn children.");
    }
}
