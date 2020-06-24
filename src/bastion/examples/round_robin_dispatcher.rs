use bastion::prelude::*;
use std::sync::Arc;

/// 
/// Dispatcher based on round robien algorithm example
fn main() {
    // Enable logs during the execution of the program
    env_logger::init();
    // We need bastion to run our program
    Bastion::init();

    Bastion::supervisor(caller_supervisor)
        .and_then(|_| Bastion::supervisor(rounder_supervisor))
        .expect("Couldn't create supervisor chain.");

    Bastion::start();
    Bastion::block_until_stopped();
}

fn rounder_supervisor(supervisor: Supervisor) -> Supervisor {
    supervisor.children(|children| rounder_group(children))
}

fn caller_supervisor(supervisor: Supervisor) -> Supervisor {
    supervisor.children(|children| caller_group(children))
}

fn rounder_group(children: Children) -> Children {
    children
        .with_redundancy(5)
        .with_dispatcher(Dispatcher::with_type(DispatcherType::Named("Rounder".to_string())))
        .with_exec(move |ctx: BastionContext| {
            async move {
                msg! {
                    ctx.recv().await?,
                    raw_message: Arc<SignedMessage> => {
                        let message = Arc::try_unwrap(raw_message).unwrap();
                        msg! { message,
                            ref data: &str => {
                                println!("Received {}", data);
                            };
                            _: _ => ();
                        }
                    };
                    _: _ => ();
                }

                Ok(())
            }
        })
}

fn caller_group(children: Children) -> Children {
    children
        .with_exec(move |ctx: BastionContext| {
            async move {
                let test: Vec<&str> = vec!["test1","test2","test3","test4","test5"];
                let target = BroadcastTarget::Group("Rounder".to_string());
                for t in test {
                    ctx.broadcast_message(target.clone(), t);
                }
                Bastion::stop();
                Ok(())
            }
        })
}