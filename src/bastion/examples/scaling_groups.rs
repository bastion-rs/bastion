use std::sync::Arc;
use std::time::Duration;

use bastion::prelude::*;
use futures_timer::Delay;

///
/// An example with the usage of the rescaling actor groups in runtime.
/// P.S. For running this example you will need to set the `scaling` feature flag.
///
/// Prologue:
/// This example demonstrates how developers can use Resizer instances for
/// auto scaling up and down actor groups based on the specified thresholds
/// for the actor mailboxes.
///
fn main() {
    Bastion::init();

    Bastion::supervisor(input_supervisor)
        .and_then(|_| Bastion::supervisor(auto_resize_group_supervisor))
        .expect("Couldn't create supervisor chain.");

    Bastion::start();
    Bastion::block_until_stopped();
}

// Supervisor that tracks only the single actor with input data
fn input_supervisor(supervisor: Supervisor) -> Supervisor {
    supervisor.children(input_group)
}

// Supervisor that tracks the actor group with rescaling in runtime.
fn auto_resize_group_supervisor(supervisor: Supervisor) -> Supervisor {
    supervisor.children(auto_resize_group)
}

#[allow(clippy::unnecessary_mut_passed)]
fn input_group(children: Children) -> Children {
    // we would have fully chained the children builder if it wasn't for the feature flag
    #[allow(unused_mut)]
    let mut children = children.with_redundancy(1);
    #[cfg(feature = "scaling")]
    {
        // Don't start new actors after finishing execution
        children =
            children.with_resizer(OptimalSizeExploringResizer::default().with_lower_bound(0));
    }
    children.with_exec(move |ctx: BastionContext| async move {
        println!("[Input] Worker started!");

        let mut messages_sent = 0;
        static INPUT: [u64; 5] = [5u64, 1, 2, 4, 3];
        let group_name = "Processing".to_string();
        let target = BroadcastTarget::Group(group_name);

        while messages_sent != 1000 {
            // Emulate the workload. The number means how
            // long it must wait before processing.
            for value in INPUT.iter() {
                ctx.broadcast_message(target.clone(), value);
                Delay::new(Duration::from_millis(75 * value)).await;
            }

            messages_sent += INPUT.len();
        }

        Ok(())
    })
}

fn auto_resize_group(children: Children) -> Children {
    // we would have fully chained the children builder if it wasn't for the feature flag
    #[allow(unused_mut)]
    let mut children = children
        .with_redundancy(3) // Start with 3 actors
        .with_heartbeat_tick(Duration::from_secs(5)); // Do heartbeat each 5 seconds

    #[cfg(feature = "scaling")]
    {
        children = children.with_resizer(
            OptimalSizeExploringResizer::default()
                .with_lower_bound(1) // A minimal acceptable size of group
                .with_upper_bound(UpperBound::Limit(10)) // Max 10 actors in runtime
                .with_upscale_strategy(UpscaleStrategy::MailboxSizeThreshold(3)) // Scale up when a half of actors have more than 3 messages
                .with_upscale_rate(0.1) // Increase the size of group on 10%, if necessary to scale up
                .with_downscale_rate(0.2), // Decrease the size of group on 20%, if too many free actors
        );
    }
    children
        .with_dispatcher(Dispatcher::with_type(DispatcherType::Named(
            "Processing".to_string(),
        )))
        .with_exec(move |ctx: BastionContext| async move {
            println!("[Processing] Worker started!");

            let mut messages_received = 0;
            let messages_limit = 25;

            while messages_received != messages_limit {
                msg! { ctx.recv().await?,
                    // We received the message from other actor wrapped in Arc<T>
                    // Let's unwrap it and do regular matching.
                    raw_message: Arc<SignedMessage> => {
                        let message = Arc::try_unwrap(raw_message).unwrap();

                        msg! { message,
                            ref number: &'static u64 => {
                                // Emulate some processing. The received number is a delay.
                                println!("[Processing] Worker #{:?} received `{}`", ctx.current().id(), number);
                                Delay::new(Duration::from_millis(**number * 500)).await;
                            };
                            _: _ => ();
                        }
                    };
                    _: _ => ();
                }

                messages_received += 1;
                println!(
                    "[Processing] Worker #{:?} processed {} message(s)",
                    ctx.current().id(),
                    messages_received
                );
            }

            println!("[Processing] Worker finished!");
            Ok(())
        })
}
