use std::collections::HashMap;
use std::sync::Arc;

use bastion::prelude::*;

///
/// An example with the usage of the broadcasting messages feature.
///
/// Prologue:
/// This example demonstrates one of the ways to organize a simple processing
/// pipeline with the help of supervised groups of actors, dispatchers and
/// broadcasting messages features.
///
/// The pipeline in this example can be described in the following way:
/// 1. The Input group contains the only one actor that stars the processing with
///    sending messages through a dispatcher to actors in the Map group.
/// 2. Each actor of the Process group does some useful work and passes a result
///    to the next stage with the similar call to the Reduce group.
/// 3. The actor from the Response group retrieves the data from the actors of the
///    Reduce group, combines the results and prints its when everything is done.
///
fn main() {
    Bastion::init();

    Bastion::supervisor(input_supervisor)
        .and_then(|_| Bastion::supervisor(map_supervisor))
        .and_then(|_| Bastion::supervisor(response_supervisor))
        .expect("Couldn't create supervisor chain.");

    Bastion::start();
    Bastion::block_until_stopped();
}

// Supervisor that tracking only the single actor with input data
fn input_supervisor(supervisor: Supervisor) -> Supervisor {
    supervisor.children(|children| input_group(children))
}

// Supervisor for actors in map group
fn map_supervisor(supervisor: Supervisor) -> Supervisor {
    supervisor.children(|children| process_group(children))
}

// Supervisor that tracking only the single actor with output data
fn response_supervisor(supervisor: Supervisor) -> Supervisor {
    supervisor.children(|children| response_group(children))
}

fn input_group(children: Children) -> Children {
    children
        .with_redundancy(1)
        .with_exec(move |ctx: BastionContext| async move {
            println!("[Input] Worker started!");

            let data = vec!["A B C", "A C C", "B C C"];
            let group_name = "Processing".to_string();
            let target = BroadcastTarget::Group(group_name);

            for input in data {
                ctx.broadcast_message(target.clone(), input);
            }

            Ok(())
        })
}

fn process_group(children: Children) -> Children {
    children
        .with_redundancy(3)
        .with_dispatcher(
            // Declare a dispatcher to use. All instantiated actors will be registered in
            // the namespace with the "Map" name and removed after being stopped or killed
            // automatically.
            //
            // If needed to use more than one groups, then do more `with_dispatcher` calls
            Dispatcher::with_type(DispatcherType::Named("Processing".to_string())),
        )
        .with_exec(move |ctx: BastionContext| async move {
            println!("[Processing] Worker started!");

            msg! { ctx.recv().await?,
                // We received the message from other actor wrapped in Arc<T>
                // Let's unwrap it and do regular matching.
                raw_message: Arc<SignedMessage> => {
                    let message = Arc::try_unwrap(raw_message).unwrap();
                    msg! { message,
                        ref data: &'static str => {
                            println!("[Processing] Worker #{:?} received `{}`", ctx.current().id(), data);

                            // Simple counter for letters in the sentence
                            let mut counter: HashMap<&str, u32> = HashMap::new();
                            for letter in data.split(' ') {
                                let value = counter.entry(letter).or_insert(0);
                                *value += 1;
                            }

                            println!("[Processing] Worker #{:?} processed data. Result: `{:?}`", ctx.current().id(), counter);

                            // Push hashmap with data to the next actor group
                            let group_name = "Response".to_string();
                            let target = BroadcastTarget::Group(group_name);
                            ctx.broadcast_message(target.clone(), counter);
                        };
                        _: _ => ();
                    }
                };
                _: _ => ();
            }

            Ok(())
        })
}

fn response_group(children: Children) -> Children {
    children
        .with_redundancy(1)
        .with_dispatcher(
            // We will re-use the dispatcher to make the example easier to understand
            // and flexibility in code.
            //
            // The single difference is only the name for Dispatcher for our actor's group.
            Dispatcher::with_type(DispatcherType::Named("Response".to_string())),
        )
        .with_exec(move |ctx: BastionContext| async move {
            println!("[Response] Worker started!");

            let mut received_messages = 0;
            let expected_messages = 3;
            let mut counter: HashMap<&str, u32> = HashMap::new();

            while received_messages != expected_messages {
                msg! { ctx.recv().await?,
                    // We received the message from other actor wrapped in Arc<T>
                    // Let's unwrap it and do regular matching.
                    raw_message: Arc<SignedMessage> => {
                        let message = Arc::try_unwrap(raw_message).unwrap();
                        msg! { message,
                            ref data: HashMap<&str, u32> => {
                                println!("[Response] Worker received `{:?}`", data);

                                for (key, value) in data.iter() {
                                    let current_value = counter.entry(key).or_insert(0);
                                    *current_value+= value;
                                }

                                received_messages += 1;
                            };
                            _: _ => ();
                        }
                    };
                    _: _ => ();
                }
            }

            println!("[Response] Aggregated data: `{:?}`", counter);
            Ok(())
        })
}
