use bastion::prelude::*;
use std::sync::{
    atomic::{AtomicUsize, Ordering},
    Arc,
};
use std::thread;
use std::time::Duration;

#[test]
fn test_run_blocking() {
    Bastion::init();
    Bastion::start();

    let c = Bastion::children(|children| {
        // We are creating the function to exec
        children.with_exec(|ctx: BastionContext| {
            let received_messages = Arc::new(AtomicUsize::new(0));
            async move {
                let received_messages = Arc::clone(&received_messages);
                loop {
                    msg! {
                        ctx.recv().await?,
                        msg: &'static str =!>
                         {
                            assert_eq!(msg, "hello");
                            let messages = received_messages.fetch_add(1, Ordering::SeqCst);
                            answer!(ctx, messages + 1).expect("couldn't reply :(");
                        };
                        _: _ => panic!();
                    }
                }
                Ok(())
            }
        })
    })
    .unwrap();

    let child = c.elems()[0].clone();

    let output = (0..100)
        .map(|_| {
            let child = child.clone();
            run!(blocking!(
                async move {
                    let duration = Duration::from_millis(1);
                    thread::sleep(duration);
                    msg! {
                        child.clone()
                            .ask_anonymously("hello").unwrap().await.unwrap(),
                            output: usize => output;
                            _: _ => panic!();
                    }
                }
                .await
            ))
            .unwrap()
        })
        .collect::<Vec<_>>();

    Bastion::stop();
    Bastion::block_until_stopped();

    assert_eq!((1..=100).map(|i| i).collect::<Vec<_>>(), output);
}
