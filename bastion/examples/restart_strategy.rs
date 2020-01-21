use std::time::Duration;

use bastion::prelude::*;

fn main() {
    Bastion::init();

    Bastion::supervisor(supervisor).expect("Couldn't create the supervisor.");

    Bastion::start();
    Bastion::block_until_stopped();
}

fn supervisor(supervisor: Supervisor) -> Supervisor {
    let restart_strategy = RestartStrategy::default()
        .with_max_restarts(Some(3))
        .with_actor_restart_strategy(ActorRestartStrategy::LinearBackOff {
            timeout: Duration::from_secs(1),
        });

    supervisor
        .with_restart_strategy(restart_strategy)
        .children(|children| failed_actors_group(children))
}

fn failed_actors_group(children: Children) -> Children {
    children.with_exec(move |_ctx: BastionContext| async move {
        println!("Worker started!");
        panic!("Unexpected error...");
    })
}
