use bastion::prelude::*;
use proptest::prelude::*;

use std::sync::Once;

static START: Once = Once::new();

proptest! {
    #![proptest_config(ProptestConfig::with_cases(1_000))]
    #[test]
    fn proptest_intra_message(message in "\\PC*") {
        START.call_once(|| {
            Bastion::init();
        });
        Bastion::start();

        match Bastion::children(|children: Children| {
            children
                .with_exec(move |ctx: BastionContext| {
                    async move {
                        msg! { ctx.recv().await?,
                            ref msg: &'static str => {
                                println!("Broadcasted :: {}", msg);
                            };
                            // This won't happen because this example
                            // only "asks" a `&'static str`...
                            _: _ => ();
                        }

                        Ok(())
                    }
                })
        }) {
            Ok(_chrn) => {
                let message: &'static str = Box::leak(message.into_boxed_str());
                Bastion::broadcast(message).expect("broadcast failed");
            },
            _ => (),
        }
    }
}
