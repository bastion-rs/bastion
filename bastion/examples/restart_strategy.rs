use std::time::Duration;

use bastion::prelude::*;

///
/// Supervisors with a custom restart strategy example.
///
/// Prologue:
/// This examples demonstrates how to override the default restart strategy
/// on a custom, provided by the bastion crate. The supervisor will spawn
/// the only one tracked actor that will be restarted a couple of times and
/// the certain timeout after a raised failure in the actor's code.
///
fn main() {
    Bastion::init();

    Bastion::supervisor(supervisor).expect("Couldn't create the supervisor.");

    Bastion::start();
    Bastion::block_until_stopped();
}

fn supervisor(supervisor: Supervisor) -> Supervisor {
    // Here we are specifying the used restart strategy for our supervisor.
    // By default the bastion's supervisors are always trying to restart
    // failed actors with unlimited amount of tries.
    //
    // At the beginning we're creating a new instance of RestartStrategy
    // and then provides a policy and a back-off strategy.
    let restart_strategy = RestartStrategy::default()
        // Set the limits for supervisor, so that it could stop
        // after 3 attempts. If the actor can't be started, the supervisor
        // will remove the failed actor from tracking.
        .with_restart_policy(RestartPolicy::Tries(3))
        // Set the desired restart strategy. By default supervisor will
        // try to restore the failed actor as soon as possible. However,
        // in our case we want to restart with a small delay between the
        // tries. Let's say that we want a regular time interval between the
        // attempts which is equal to 1 second.
        .with_actor_restart_strategy(ActorRestartStrategy::LinearBackOff {
            timeout: Duration::from_secs(1),
        });

    // After it we define the supervisor...
    supervisor
        // That uses our restart strategy defined earlier
        .with_restart_strategy(restart_strategy)
        // And tracks the child group, defined in the following function
        .children(|children| failed_actors_group(children))
}

fn failed_actors_group(children: Children) -> Children {
    // Specifying the child group, where each actor
    // will output the sentence in the stdout, then will fail
    // with panic.
    children.with_exec(move |_ctx: BastionContext| async move {
        println!("Worker started!");
        panic!("Unexpected error...");
    })
}
