use bastion::prelude::*;
use proptest::prelude::*;
use std::sync::Arc;
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

        let message = Arc::new(message);

        let _ = Bastion::children(|children| {
            children
                .with_exec(move |ctx: BastionContext| {
                    let message = (*message).clone();
                    async move {
                        let message: &'static str = Box::leak(message.into_boxed_str());
                        let answer = ctx
                            .ask(&ctx.current().addr(), message)
                            .expect("Couldn't send the message.");

                        msg! { ctx.recv().await?,
                            msg: &'static str =!> {
                                let _ = answer!(ctx, msg);
                            };
                            _: _ => ();
                        }

                        msg! { answer.await?,
                            _msg: &'static str => {};
                            _: _ => {};
                        }

                        Ok(())
                    }
                })
        });
    }
}
