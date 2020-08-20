use bastion::supervisor::{ActorRestartStrategy, RestartPolicy, RestartStrategy};
use std::time::{Duration, Instant};

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

#[test]
fn calculate_immediate_strategy() {
    let strategy = ActorRestartStrategy::Immediate;

    assert_eq!(strategy.calculate(0), None);
    assert_eq!(strategy.calculate(1), None);
    assert_eq!(strategy.calculate(100), None);
}

#[test]
fn calculate_linear_strategy() {
    let strategy = ActorRestartStrategy::LinearBackOff {
        timeout: Duration::from_millis(100),
    };

    assert_eq!(strategy.calculate(0), Some(Duration::from_millis(100)));

    assert_eq!(
        strategy.calculate(1),
        Some(Duration::from_millis(100 + 1 * 100))
    );
    assert_eq!(
        strategy.calculate(99),
        Some(Duration::from_millis(100 + 99 * 100))
    );
}

#[test]
fn calculate_exp_strategy_with_multiplier_zero() {
    let strategy = ActorRestartStrategy::ExponentialBackOff {
        timeout: Duration::from_millis(100),
        multiplier: 0.0,
    };

    assert_eq!(strategy.calculate(0), Some(Duration::from_millis(100)));
    assert_eq!(strategy.calculate(1), Some(Duration::from_millis(100)));
    assert_eq!(strategy.calculate(100), Some(Duration::from_millis(100)));
}

#[test]
fn calculate_exp_strategy_with_multiplier_non_zero() {
    let strategy = ActorRestartStrategy::ExponentialBackOff {
        timeout: Duration::from_millis(100),
        multiplier: 5.0,
    };

    assert_eq!(strategy.calculate(0), Some(Duration::from_millis(100)));

    assert_eq!(
        strategy.calculate(1),
        Some(Duration::from_millis(100 + 1 * 5 * 100))
    );
    assert_eq!(
        strategy.calculate(99),
        Some(Duration::from_millis(100 + 99 * 5 * 100))
    );
}
