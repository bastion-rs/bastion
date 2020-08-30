use bastion::prelude::*;
use futures_timer::Delay;
use lightproc::proc_state::State;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Mutex;
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

    let state: Arc<Mutex<dyn State>> = Arc::new(Mutex::new(StateWithCounter::default()));

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
                        msg: usize => {
                            tracing::error!("YAY");
                        };
                        msg: usize => {
                            tracing::error!("YAY");
                        };
                        msg: &usize => {
                            tracing::error!("&YAY");
                        };
                        ref msg: usize => {
                            tracing::error!("ref YAY");
                        };
                        ref msg: &usize => {
                            tracing::error!("ref &YAY");
                        };
                        ref msg: &str => {
                            match msg {
                                &"increment" => {
                                    tracing::error!("INCREMENT!!!");
                                    ctx.with_state(|state: &StateWithCounter| state.increment()).await;
                                },
                                &"get" => {
                                    let mut counter = 0;
                                    ctx.with_state(|state: &StateWithCounter| {counter = state.get();}).await;
                                    tracing::error!("GET {}", counter);
                                    // answer!(ctx, format!("the counter is {}", counter)).unwrap();
                                },
                                _ => {
                                    tracing::error!("NOPE");
                                }
                           }
                        };
                        msg: &str => {
                            match msg {
                                "increment" => {
                                    tracing::error!("INCREMENT!!!");
                                    ctx.with_state(|state: &StateWithCounter| state.increment()).await;
                                },
                                "get" => {
                                    let mut counter = 0;
                                    ctx.with_state(|state: &StateWithCounter| {counter = state.get();}).await;
                                    tracing::error!("GET {}", counter);
                                    // answer!(ctx, format!("the counter is {}", counter)).unwrap();
                                },
                                _ => {
                                    tracing::error!("NOPE");
                                }
                           }
                        };
                        // We define the behavior when we receive a new msg
                        ref raw_message: SignedMessage => {
                            panic!("aw")
                        };
                        // We define the behavior when we receive a new msg
                        raw_message: Arc<SignedMessage> => {
                            panic!("no")

                            // // We open the message
                            // msg! { Arc::try_unwrap(raw_message).unwrap(),
                            //     msg: usize => {
                            //         tracing::error!("sub YAY");
                            //     };
                            //     msg: &usize => {
                            //         tracing::error!("sub &YAY");
                            //     };
                            //     ref msg: usize => {
                            //         tracing::error!("sub ref YAY");
                            //     };
                            //     ref msg: &usize => {
                            //         tracing::error!("sub ref &YAY");
                            //     };
                            //     ref msg: &str => {
                            //         match msg {
                            //             &"increment" => {
                            //                 tracing::error!("INCREMENT!!!");
                            //                 ctx.with_state(|state: &StateWithCounter| state.increment()).await;
                            //             },
                            //             &"get" => {
                            //                 let mut counter = 0;
                            //                 ctx.with_state(|state: &StateWithCounter| {counter = state.get();}).await;
                            //                 tracing::error!("GET {}", counter);
                            //                 // answer!(ctx, format!("the counter is {}", counter)).unwrap();
                            //             },
                            //             _ => {
                            //                 tracing::error!("NOPE");
                            //             }
                            //        }
                            //     };
                            //     msg: &str => {
                            //         tracing::error!("msg!");
                            //         match msg {
                            //             "increment" => {
                            //                 tracing::error!("INCREMENT!!!");
                            //                 ctx.with_state(|state: &StateWithCounter| state.increment()).await;
                            //             },
                            //             "get" => {
                            //                 let mut counter = 0;
                            //                 ctx.with_state(|state: &StateWithCounter| {counter = state.get();}).await;
                            //                 tracing::error!("GET {}", counter);
                            //                 // answer!(ctx, format!("the counter is {}", counter)).unwrap();
                            //             },
                            //             _ => {
                            //                 tracing::error!("NOPE");
                            //             }
                            //        }
                            //     };
                            //     _: _ => {panic!("neither"); ()};
                            // }
                        };
                        _: _ => {tracing::error!("nope"); ()};
                    }
                }
            },
        )
    }))
    .and_then(|_| Bastion::supervisor(|supervisor| supervisor.children(|children| {
        children.with_exec(move |ctx: BastionContext| async move {
            let target = BroadcastTarget::Group("counter".to_string());
            for _ in 0..1000 {
                ctx.tell_one(target.clone(), 1usize);
                Delay::new(Duration::from_millis(200)).await;
                ctx.tell_all(target.clone(), 1usize);
                Delay::new(Duration::from_millis(200)).await;

                ctx.tell_one(target.clone(), &1usize);
                Delay::new(Duration::from_millis(200)).await;

                // ctx.broadcast_message(target.clone(), &1usize);

                Delay::new(Duration::from_millis(200)).await;

                // ctx.broadcast_message(target.clone(), 1usize);

                ctx.tell_one(target.clone(), "get");
                // msg! {
                //     ctx.recv().await?,
                //     ref answer: &str => {
                //         tracing::warn!("Received {}", answer);
                //     };
                //     _: _ => ();
                // };

                Delay::new(Duration::from_millis(200)).await;

                ctx.tell_one(target.clone(), "increment");

                Delay::new(Duration::from_millis(200)).await;
            }
            Ok(())
        })
    })))
    .expect("Couldn't create supervisor chain.");

    Bastion::start();
    Bastion::block_until_stopped();
}
