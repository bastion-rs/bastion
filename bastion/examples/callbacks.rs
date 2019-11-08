use bastion::prelude::*;

fn main() {
    Bastion::init();

    Bastion::supervisor(|supervisor| {
        let children_ref = supervisor.children_ref(|children| {
            let callbacks = Callbacks::new()
                .with_before_start(|| {
                    // This children group is first in order, so its `before_start` will
                    // get called before the other one's...
                    println!("1: before_start");
                })
                .with_before_restart(|| {
                    // This children group is first in order and the other children group
                    // stopped (thus, its `after_stop` has already been called and
                    // `before_restart` won't be)...
                    println!("9: before_restart");
                })
                .with_after_restart(|| {
                    // This children group is first in order, so its `after_restart` will
                    // get called before the other one's...
                    println!("10: after_restart");

                    // The other children group has not been restarted yet...
                    println!("11: stop");
                    // This will stop both children group...
                    Bastion::stop();
                })
                .with_after_stop(|| {
                    // This will get called after the other children group has been restarted
                    // (because the supervisor will then get a message from the system saying
                    // that it needs to stop running)...
                    println!("13: after_stop");
                });

            children
                .with_exec(|ctx| {
                    async move {
                        // This might not get called before the other child's `stop`
                        // (this is up to the executor) and/or `recv`...
                        println!("3|4|5: recv");
                        // This will await for the other child to be stopped
                        // and to call `tell`...
                        ctx.recv().await.expect("Couldn't receive the message.");

                        // The other child has stopped and sent a message to stop
                        // this child from awaiting...
                        println!("8: err");
                        // This will make both children group get restarted...
                        Err(())
                    }
                })
                .with_callbacks(callbacks)
        });

        supervisor
            .with_strategy(SupervisionStrategy::OneForAll)
            .children(|children| {
                let callbacks = Callbacks::new()
                    .with_before_start(|| {
                        // This children group is second in order, so its `before_start` will
                        // get called after the other one's...
                        println!("2: before_start");
                    })
                    .with_before_restart(|| {
                        // This won't happen because this child never faults...
                        unreachable!();
                    })
                    .with_after_restart(|| {
                        // This children group is second in order, so its `after_restart` will
                        // get called after the other one's...
                        println!("12: after_restart");
                    })
                    .with_after_stop(move || {
                        // This will get called both after this child's `recv` (see there why)
                        // and after the other children group's `stop` (which stops the system)...
                        println!("6|14: after_stop");

                        // Nothing will get printed in between because the other child's `exec`
                        // future is pending...
                        println!("7|15: tell");
                        // This will "tell" a message to the other child, making it finish
                        // `await`ing on `ctx.recv()` and return an error...
                        children_ref.elems()[0].tell(()).ok();
                    });

                children
                    .with_exec(|ctx| {
                        async move {
                            // This might not get called before the other child's `recv`
                            // (this is up to the executor)...
                            println!("3|4|5: stop");
                            // This will stop this children gruop once the future becomes
                            // pending...
                            ctx.parent()
                                .stop()
                                .expect("Couldn't stop the children group.");

                            // This might not get called before the other child's `recv`
                            // (this is up to the executor)...
                            println!("4|5: recv");
                            // This will make the future pending and allow it to stop (because
                            // `ctx.current().stop()` was called earlier)...
                            ctx.recv().await.expect("Couldn't receive the message.");

                            // Note that this will never get there...
                            Ok(())
                        }
                    })
                    .with_callbacks(callbacks)
            })
    })
        .expect("Couldn't create the supervisor.");

    Bastion::start();
    Bastion::block_until_stopped();
}
