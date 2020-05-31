use bastion::prelude::*;
use std::fmt::{Display, Formatter, Result};
use std::sync::Arc;
use tracing::{error, info};

#[derive(Debug)]
struct Ping {
    count: usize,
}

impl Display for Ping {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        write!(f, "Ping ! {}", self.count)
    }
}

#[derive(Debug)]
struct Pong {
    count: usize,
}

impl Display for Pong {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        write!(f, "Pong ! {}", self.count)
    }
}

///
/// Ping pong job example
///
/// Prologue:
/// This example will show two children cooperating by sending pings and pongs until MAX is reached.
fn main() {
    env_logger::init();

    Bastion::init();

    // children that send pongs.
    let pong_children = Bastion::children(|children: Children| {
        children
            .with_redundancy(10)
            .with_exec(move |ctx: BastionContext| {
                async move {
                    info!("pong child ready!");
                    // Start receiving work
                    loop {
                        msg! { ctx.recv().await?,
                            msg: Ping =!> {
                                let response = Pong {
                                    count: msg.count + 1
                                };
                                println!("{}", msg);
                                let _ = answer!(ctx, response);
                            };
                            _: _ => {
                                println!("Pong child received a message it can't process");
                            };
                        }
                    }
                }
            })
    })
    .expect("Couldn't start a new children group.");

    let pong_children = Arc::new(pong_children);

    dbg!(&pong_children.dispatchers());

    // children that send pings.
    let ping_children = Bastion::children(|children: Children| {
        children
            .with_redundancy(10)
            .with_exec(move |ctx: BastionContext| {
                let pong_children = Arc::clone(&pong_children);
                async move {
                    println!("ping child ready!");
                    // Start receiving work
                    loop {
                        msg! { ctx.recv().await?,
                            ref msg: &'static str => {
                                if msg == &"start" {
                                    println!("leggo");
                                    (*pong_children).ask(Ping { count: 0});
                                }
                            };
                            msg: Pong =!> {
                                let response = Ping {
                                    count: msg.count + 1
                                };
                                println!("{}", msg);
                                let _ = answer!(ctx, response);
                            };
                            _: _ => {
                                println!("Pong child received a message it can't process");
                            };
                        }
                    }
                }
            })
    })
    .expect("Couldn't start a new children group.");

    Bastion::start();

    let handle = std::thread::spawn(move || {
        println!("------------------let s do stuff");

        std::thread::sleep_ms(5000);
        println!("------------------let s start");
        ping_children.tell("start");
    });
    handle.join().unwrap();

    Bastion::block_until_stopped();
}
