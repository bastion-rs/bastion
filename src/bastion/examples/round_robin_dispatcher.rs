use bastion::prelude::*;

/// 
/// Dispatcher based on round robien algorithm example
fn main() {
    // Enable logs during the execution of the program
    env_logger::init();
    // We need bastion to run our program
    Bastion::init();

    let children = Bastion::children(|children: Children| {
        children
            .with_redundancy(5)
            .with_exec(move |ctx: BastionContext| {
                async move {
                    println!("Test");
                    msg! {
                        ctx.recv().await?,
                        ref _msg: &'static str => {
                            println!("static msg ref");
                        };
                        _msg: &'static str => {
                            println!("static msg 1");
                        };
                        _: _ => ();
                    }

                    Ok(())
                }
            })
    }).expect("Couldn't start a new children group");

    Bastion::start();
}