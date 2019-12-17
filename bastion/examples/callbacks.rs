use bastion::prelude::*;

fn main() {
    env_logger::init();

    Bastion::init();

    Bastion::supervisor(sp).expect("Couldn't create the supervisor.");

    Bastion::start();
    Bastion::block_until_stopped();
}

fn sp(supervisor: Supervisor) -> Supervisor {
    let callbacks = Callbacks::new()
        .with_before_start(|| {
            // This will get called before others `before_start`s of this example
            // because the others all are defined for elements supervised by
            // this supervisor...
            println!("(sp      ) before_start");
        })
        .with_after_stop(|| {
            // This will get called after others `after_stop`s of this example
            // because the others all are defined for elements supervised by
            // this supervisor...
            println!("(sp      ) after_stop");
        });

    let children_ref = supervisor.children_ref(sp_ch);

    supervisor
        .supervisor(|sp| sp_sp(sp, children_ref))
        .with_strategy(SupervisionStrategy::OneForAll)
        .with_callbacks(callbacks)
}

fn sp_ch(children: Children) -> Children {
    let callbacks = Callbacks::new()
        .with_before_start(|| {
            // This will be the first `before_start` callback to get called after
            // "sp"'s because this is its first supervised element in order...
            println!("(sp.ch   ) before_start");
        })
        .with_before_restart(|| {
            // This will be the first `before_restart` callback to get called
            // after "sp" restarts its supervised elements because this is its
            // first supervised element in order...
            println!("(sp.ch   ) before_restart");
        })
        .with_after_restart(|| {
            // This might get called before, after or in-between "sp.sp.ch"'s
            // `before_start` and "sp.sp"'s `after_restart`...
            println!("(sp.ch   ) after_restart");

            // This might get called before, after or in-between "sp.sp.ch"'s
            // `before_start` and "sp.sp"'s `after_restart`...
            println!("(sp.ch   ) stop");
            // This will stop the whole system (once "sp" finished restarting its
            // supervised elements and both "sp.ch" and "sp.sp.ch"'s futures
            // become pending)...
            Bastion::stop();
        })
        .with_after_stop(|| {
            // This will get called after `recv` but might get called before,
            // after or in-between "sp.sp"'s `after_stop` and "sp.sp.ch"'s
            // `after_stop`...
            println!("(sp.ch   ) after_stop");
        });

    children
        .with_exec(|ctx| {
            async move {
                // This will get called two times:
                // - a first time after `before_start` but before, after or
                //   in-between "sp.sp"'s `before_start and "sp.sp.ch"'s `before_start`,
                //   `stop`, `recv` and `after_stop`...
                // - a second time after `after_restart` but before, after or
                //   in-between "sp.sp"'s `before_restart`, `after_restart` and
                //   `after_stop` and "sp.sp.ch"'s `before_start`, `stop`, `recv` and
                //   `after_stop`...
                println!("(sp.ch   ) recv");
                // This will wait for the message sent by "sp.sp.ch"'s `tell` (when
                // its `after_stop` gets called)...
                ctx.recv().await.expect("Couldn't receive the message.");

                // Once the message has been received, the future will return an
                // error to make "sp" restart "sp.ch" and "sp.sp" (because its
                // supervision strategy is "one-for-all")...
                println!("(sp.ch   ) err");
                Err(())
            }
        })
        .with_callbacks(callbacks)
}

fn sp_sp(supervisor: Supervisor, children_ref: ChildrenRef) -> Supervisor {
    let callbacks = Callbacks::new()
        .with_before_start(|| {
            // This will get called after "sp.ch"'s `before_start` and might get
            // called before or after its `recv`...
            println!("(sp.sp   ) before_start");
        })
        .with_before_restart(|| {
            // This will get called after "sp.ch"'s `before_restart`...
            println!("(sp.sp   ) before_restart");
        })
        .with_after_restart(|| {
            // This will get called after "sp.sp.ch"'s `before_start` and might
            // get called before, after or in-between its `recv` and "sp.ch"'s
            // `recv`...
            println!("(sp.sp   ) after_restart");
        })
        .with_after_stop(|| {
            // This will get called after "sp.sp.ch"'s `after_stop` but might get
            // called before or after "sp.ch"'s `recv`...
            println!("(sp.sp   ) after_stop");
        });

    supervisor
        .children(|sp| sp_sp_ch(sp, children_ref))
        .with_callbacks(callbacks)
}

fn sp_sp_ch(children: Children, children_ref: ChildrenRef) -> Children {
    let callbacks = Callbacks::new()
        .with_before_start(|| {
            // This will get called two times:
            // - a first time after "sp.sp"'s `before_start` and before or after
            //   "sp.ch"'s `recv`
            // - a second time after "sp.sp"'s `before_restart` and before, after
            //   or in-between "sp.ch"'s `after_restart` and `stop`...
            println!("(sp.sp.ch) before_start");
        })
        // This won't get called because this children group only stop itself
        // (thus, `after_stop` would have already been called)...
        .with_before_restart(|| unreachable!())
        // This won't get called because this children group only stops itself
        // (thus, `before_start` will get called instead)...
        .with_after_restart(|| unreachable!())
        .with_after_stop(move || {
            // This will get called two times, both after `recv` but before or
            // after "sp.ch"'s `recv`...
            println!("(sp.sp.ch) after_stop");

            // This might get called before or after "sp.ch"'s `recv`...
            println!("(sp.sp.ch) tell");
            // This will send a message to "sp.ch", making it fault and making
            // "sp" restart "sp.ch" and "sp.sp" (because its supervision strategy
            // is "one-for-one")...
            children_ref.elems()[0].tell_anonymously(()).ok();
        });

    children
        .with_exec(|ctx| {
            async move {
                // This will get called two times, both after `before_start` and
                // before or after "sp.ch"'s `recv`...
                println!("(sp.sp.ch) stop");
                // This will stop this children group once this future becomes
                // pending...
                ctx.parent()
                    .stop()
                    .expect("Couldn't stop the children group.");

                // This might get called before or after "sp.ch"'s `recv`...
                println!("(sp.sp.ch) recv");
                // This will make this future pending, thus allowing the children
                // group to stop...
                ctx.recv().await.expect("Couldn't receive the message.");

                // Note that this future will never get there...
                Ok(())
            }
        })
        .with_callbacks(callbacks)
}
