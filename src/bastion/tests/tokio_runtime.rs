#[cfg(feature = "runtime-tokio")]
mod tokio_tests {

    use bastion::prelude::*;

    #[tokio::test]
    async fn test_simple_await() {
        tokio::time::sleep(std::time::Duration::from_nanos(1)).await;
    }

    #[tokio::test]
    async fn test_within_children() {
        Bastion::init();
        Bastion::start();

        Bastion::children(|children| {
            children.with_exec(|_| async move {
                tokio::time::sleep(std::time::Duration::from_nanos(1)).await;
                Ok(())
            })
        })
        .expect("Couldn't create the children group.");

        Bastion::stop();
    }

    #[tokio::test]
    async fn test_within_message_receive() {
        Bastion::init();
        Bastion::start();

        let workers = Bastion::children(|children| {
            children.with_exec(|ctx| async move {
                msg! {
                    ctx.recv().await?,
                    question: &'static str =!> {
                        if question != "marco" {
                            panic!("didn't receive expected message");
                        }
                        tokio::time::sleep(std::time::Duration::from_nanos(1)).await;
                        answer!(ctx, "polo").expect("couldn't send answer");
                    };
                    _: _ => {
                        panic!("didn't receive &str");
                    };
                }
                Ok(())
            })
        })
        .expect("Couldn't create the children group.");

        let answer = workers.elems()[0]
            .ask_anonymously("marco")
            .expect("Couldn't send the message.");

        msg! { answer.await.expect("couldn't receive answer"),
            reply: &'static str => {
                if reply != "polo" {
                    panic!("didn't receive expected message");
                }
            };
            _: _ => { panic!("didn't receive &str"); };
        }
        Bastion::stop();
    }

    #[tokio::test]
    async fn test_within_message_receive_blocking() {
        Bastion::init();
        Bastion::start();

        let workers = Bastion::children(|children| {
            children.with_exec(|ctx| async move {
                msg! {
                    ctx.recv().await?,
                    question: &'static str =!> {
                        if question != "marco" {
                            panic!("didn't receive expected message");
                        }
                        run!(blocking! {
                            let _ = tokio::time::sleep(std::time::Duration::from_nanos(1)).await;
                            println!("done");
                        });
                        answer!(ctx, "polo").expect("couldn't send answer");
                    };
                    _: _ => {
                        panic!("didn't receive &str");
                    };
                }
                Ok(())
            })
        })
        .expect("Couldn't create the children group.");

        let answer = workers.elems()[0]
            .ask_anonymously("marco")
            .expect("Couldn't send the message.");

        msg! { answer.await.expect("couldn't receive answer"),
            reply: &'static str => {
                if reply != "polo" {
                    panic!("didn't receive expected message");
                }
            };
            _: _ => { panic!("didn't receive &str"); };
        }
        Bastion::stop();
    }

    #[tokio::test]
    async fn test_within_message_receive_spawn() {
        Bastion::init();
        Bastion::start();

        let workers = Bastion::children(|children| {
            children.with_exec(|ctx| async move {
                msg! {
                    ctx.recv().await?,
                    question: &'static str =!> {
                        if question != "marco" {
                            panic!("didn't receive expected message");
                        }
                        run!(blocking! {
                            let _ = tokio::time::sleep(std::time::Duration::from_nanos(1)).await;
                            println!("done");
                        });
                        answer!(ctx, "polo").expect("couldn't send answer");
                    };
                    _: _ => {
                        panic!("didn't receive &str");
                    };
                }
                Ok(())
            })
        })
        .expect("Couldn't create the children group.");

        let answer = workers.elems()[0]
            .ask_anonymously("marco")
            .expect("Couldn't send the message.");

        msg! { answer.await.expect("couldn't receive answer"),
            reply: &'static str => {
                if reply != "polo" {
                    panic!("didn't receive expected message");
                }
            };
            _: _ => { panic!("didn't receive &str"); };
        }
        Bastion::stop();
    }
}
