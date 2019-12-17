use bastion::prelude::*;
use std::sync::Arc;

///
/// Parallel computation (MapReduce) job example
///
/// Prologue:
/// This example maps a stream of cycling values([0,1,2,3,4,0,1,2,3,4...]) one by one:
/// to 10 workers and every worker compute the double of what they receive and send back.
///
/// Then mapper aggregates the doubled values and sum them.
///
/// Try increasing the worker count. Yes!
fn main() {
    env_logger::init();

    Bastion::init();

    // Workers that process the work.
    let workers = Bastion::children(|children: Children| {
        children
            .with_redundancy(10) // Let's have a pool of ten workers.
            .with_exec(move |ctx: BastionContext| {
                async move {
                    println!("Worker started!");

                    // Start receiving work
                    loop {
                        msg! { ctx.recv().await?,
                            msg: u64 =!> {
                                let data: u64 = msg.wrapping_mul(2);
                                println!("Child doubled the value of {} and gave {}", msg, data); // true
                                let _ = answer!(ctx, data);
                            };
                            _: _ => ();
                        }
                    }
                }
            })
    })
    .expect("Couldn't start a new children group.");

    // Get a shadowed sharable reference of workers.
    let workers = Arc::new(workers);

    //
    // Mapper that generates work.
    Bastion::children(|children: Children| {
        children.with_exec(move |ctx: BastionContext| {
            let workers = workers.clone();
            async move {
                println!("Mapper started!");

                // Asynchronous computation result will be stored here.
                let mut sum_of_doubles = 0_u64;

                // Distribute your workload to workers
                for id_worker_pair in workers.elems().iter().enumerate() {
                    let data = cycle(id_worker_pair.0 as u64, 5);

                    let computed: Answer = ctx.ask(&id_worker_pair.1.addr(), data).unwrap();
                    msg! { computed.await?,
                        msg: u64 => {
                            // Handle the answer...
                            println!("Source received the computed value: {}", msg);
                            sum_of_doubles += msg;
                        };
                        _: _ => ();
                    }
                }

                // Tell result of our parallel computation's output.
                println!("Worker computed sum is {}", sum_of_doubles);

                // Send a signal to system that computation is finished.
                Bastion::stop();

                Ok(())
            }
        })
    })
    .expect("Couldn't start a new children group.");

    Bastion::start();
    Bastion::block_until_stopped();
}

fn cycle(x: u64, at_most: u64) -> u64 {
    let mut x = x;
    x += 1;
    x % at_most
}
