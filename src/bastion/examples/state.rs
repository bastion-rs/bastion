use async_mutex::Mutex;
use bastion::prelude::*;
use futures_timer::Delay;
use lightproc::proc_state::State;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::{sync::Arc, time::Duration};

struct StateWithCounter {
    counter: AtomicUsize,
}

impl Default for StateWithCounter {
    fn default() -> Self {
        Self {
            counter: Default::default(),
        }
    }
}

impl StateWithCounter {
    pub fn get(&self) -> usize {
        self.counter.load(Ordering::SeqCst)
    }

    pub fn increment(&self) {
        self.counter.fetch_add(1, Ordering::SeqCst);
    }
}

fn main() {
    let subscriber = tracing_subscriber::fmt()
        .with_max_level(tracing::Level::WARN)
        .finish();
    tracing::subscriber::set_global_default(subscriber).unwrap();
    Bastion::init();

    let state: Arc<Mutex<Box<dyn State>>> =
        Arc::new(Mutex::new(Box::new(StateWithCounter::default())));

    std::thread::sleep(Duration::from_secs(1));

    Bastion::supervisor(|supervisor| supervisor.children(|children| {
        children
        .with_dispatcher(Dispatcher::with_type(DispatcherType::Named(
            "counter".to_string(),
        )))
        .with_state(state)
        .with_redundancy(2)
        .with_exec(
            move |ctx: BastionContext| async move {
                loop {
                    msg! { ctx.recv().await?,
                        // We define the behavior when we receive a new msg
                        raw_message: Arc<SignedMessage> => {
                            // We open the message
                            let message = Arc::try_unwrap(raw_message).unwrap();
                            msg! { message,
                                ref msg: &str => {
                                    match msg {
                                        &"increment" => {
                                            let mut state =  ctx.state().unwrap().lock().await;
                                        let dyn_state = state.as_any();
                                        let downcasted = dyn_state.downcast_ref::<StateWithCounter>().unwrap();
                                        downcasted.increment();
                                        },
                                        &"get" => {
                                            let mut state =  ctx.state().unwrap().lock().await;
                                            let dyn_state = state.as_any();
                                            let downcasted = dyn_state.downcast_ref::<StateWithCounter>().unwrap();
                                            let counter = downcasted.get();
                                            tracing::error!("GET {}", counter);
                                            // answer!(ctx, format!("the counter is {}", counter)).unwrap();
                                        },
                                        _ => {
                                            tracing::error!("NOPE");
                                        }
                                   }
                                };
                                _: _ => {panic!(); ()};
                            }
                        };
                        _: _ => ();
                    }
                }
            },
        )
    }))
    .and_then(|_| Bastion::supervisor(|supervisor| supervisor.children(|children| {
        children.with_exec(move |ctx: BastionContext| async move {
            let target = BroadcastTarget::Group("counter".to_string());
            for _ in 0..1000 {
                ctx.broadcast_message(target.clone(), "get");
                // msg! {
                //     ctx.recv().await?,
                //     ref answer: &str => {
                //         tracing::warn!("Received {}", answer);
                //     };
                //     _: _ => ();
                // };

                Delay::new(Duration::from_millis(200)).await;

                ctx.broadcast_message(target.clone(), "increment");

                Delay::new(Duration::from_millis(200)).await;
            }
            Ok(())
        })
    })))
    .expect("Couldn't create supervisor chain.");

    Bastion::start();
    Bastion::block_until_stopped();
}
