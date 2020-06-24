use bastion::prelude::*;
use std::sync::Arc;

/// 
/// Dispatcher based on round robien algorithm example
fn main() {
    // Enable logs during the execution of the program
    env_logger::init();
    // We need bastion to run our program
    Bastion::init();
    Bastion::start();

    Bastion::children(|children: Children| {
        children
            .with_name("dispatcher")
            .with_redundancy(5)
            .with_dispatcher(Dispatcher::with_type(DispatcherType::Named("Roudner".to_string())))
            .with_exec(move |ctx: BastionContext| {
                async move {
                    msg! { 
                        ctx.recv().await?,
                        t: &'static str =!> {
                            println!("Received {} str", t);
                            answer!(ctx, t).expect("Couldn't send an answer.");
                        };
                        _: _ => ();
                    }

                    Ok(())
                }
            })
    }).expect("Couldn't start the dispatcher children group");

    let test: Vec<&'static str> = vec!["test1","test2","test3","test4"];

    Bastion::children(|children: Children| {
        children
            .with_exec(move |ctx: BastionContext| {
                let test = test.clone();
                async move {
                    let target = BroadcastTarget::Group("dispatcher".to_string());
                    for t in test {
                        ctx.broadcast_message(target.clone(), t);
                    }
                    Bastion::stop();

                    Ok(())
                }
            })
    }).expect("Couldn't start a new children group");

    Bastion::block_until_stopped();
}