use bastion_executor::blocking;
use bastion_executor::run::run;
use lightproc::proc_stack::ProcStack;
use std::thread;
use std::time::Duration;

#[cfg(feature = "tokio-runtime")]
mod tokio_tests {
    #[tokio::test]
    async fn test_run_blocking() {
        super::run_test()
    }
}

#[cfg(not(feature = "tokio-runtime"))]
mod no_tokio_tests {
    #[test]
    fn test_run_blocking() {
        super::run_test()
    }
}

fn run_test() {
    let output = run(
        blocking::spawn_blocking(
            async {
                let duration = Duration::from_millis(1);
                thread::sleep(duration);
                42
            },
            ProcStack::default(),
        ),
        ProcStack::default(),
    )
    .unwrap();

    assert_eq!(42, output);
}
