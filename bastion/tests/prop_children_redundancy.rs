use bastion::prelude::*;
use proptest::prelude::*;

proptest! {
    #[test]
    fn proptest_redundancy(r in std::usize::MIN..5_000) {
        dbg!(r);
        Bastion::children(|children| {
            children
                // shrink over the redundancy
                .with_redundancy(r)
                .with_exec(|_ctx: BastionContext| {
                    async move {
                        loop {}
                        Ok(())
                    }
                })
        });
    }
}
