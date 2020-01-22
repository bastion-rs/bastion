use bastion::supervisor::{ActorRestartStrategy, RestartPolicy, RestartStrategy};
use std::time::Duration;

#[test]
fn check_default_values() {
    let restart_strategy = RestartStrategy::default();

    assert_eq!(restart_strategy.restart_policy(), RestartPolicy::Always);
    assert_eq!(restart_strategy.strategy(), ActorRestartStrategy::Immediate);
}

#[test]
fn override_restart_policy() {
    let restart_strategy = RestartStrategy::default().with_restart_policy(RestartPolicy::Never);

    assert_eq!(restart_strategy.restart_policy(), RestartPolicy::Never);
    assert_eq!(restart_strategy.strategy(), ActorRestartStrategy::Immediate);
}

#[test]
fn override_restart_strategy() {
    let strategy = ActorRestartStrategy::LinearBackOff {
        timeout: Duration::from_secs(1),
    };

    let restart_strategy = RestartStrategy::default().with_actor_restart_strategy(strategy.clone());

    assert_eq!(restart_strategy.restart_policy(), RestartPolicy::Always);
    assert_eq!(restart_strategy.strategy(), strategy);
}

#[test]
fn override_restart_strategy_and_policy() {
    let policy = RestartPolicy::Tries(3);
    let strategy = ActorRestartStrategy::LinearBackOff {
        timeout: Duration::from_secs(1),
    };

    let restart_strategy = RestartStrategy::default()
        .with_restart_policy(policy.clone())
        .with_actor_restart_strategy(strategy.clone());

    assert_eq!(restart_strategy.restart_policy(), policy);
    assert_eq!(restart_strategy.strategy(), strategy);
}
