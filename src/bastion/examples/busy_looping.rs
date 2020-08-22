use bastion::prelude::*;
use std::sync::Arc;
use tracing::Level;

#[derive(Debug)]
struct DemoMessage {
    i: usize,
}

fn fib(n: usize) -> usize {
    if n == 0 || n == 1 {
        n
    } else {
        fib(n - 1) + fib(n - 2)
    }
}

fn main() {
    let subscriber = tracing_subscriber::fmt()
        // all spans/events with a level higher than INFO
        // will be written to stdout.
        .with_max_level(Level::ERROR)
        // completes the builder and sets the constructed `Subscriber` as the default.
        .finish();
    tracing::subscriber::set_global_default(subscriber).unwrap();

    Bastion::init();
    Bastion::start();

    let mut receiver_ref = None;

    Bastion::supervisor(|sp| {
        let sp = sp.with_strategy(SupervisionStrategy::OneForOne);
        receiver_ref = Some(
            sp.children_ref(move |children| {
                children
                    .with_name("Receiver")
                    .with_exec(move |ctx| async move {
                        let ctx = Arc::new( ctx);
                        loop {
                            let inner_ctx = Arc::clone( &ctx);
                            if let Some(msg) = ctx.try_recv().await {
                                msg! { msg,
                                    msg: DemoMessage =!> {
                                        eprintln!("DemoMessage!");
                                        let before = std::time::Instant::now();
                                        let f = {
                                            use rand::Rng;
                                            let mut rng = rand::thread_rng();
                                            let y: f64 = rng.gen::<f64>();
                                            (y* 10.) as usize + 30
                                        };
                                        eprintln!("f is {}", f);
                                        blocking!(fib(f)).await.map(|res| {
                                            eprintln!("{} computed in {}ms", res, before.elapsed().as_millis());
                                            answer!(*inner_ctx, msg.i).unwrap();
                                        });
                                    };
                                    _:_ => {
                                        eprintln!("invalid message received");
                                        panic!("Invalid message received");
                                    };
                                }
                            } else {
                            }
                        }
                    })
            })
            .elems()[0]
                .clone(),
        );
        sp
    })
    .unwrap();

    let mut i = 0;
    loop {
        let clone = receiver_ref.clone();
        run!(blocking!(async move {
            std::thread::sleep(std::time::Duration::from_millis(100));
            msg! { clone
            .unwrap()
            .ask_anonymously(DemoMessage{ i })
            .unwrap()
            .await
            .unwrap(),
                response: usize => {
                    eprintln!("Response = {:?}", response);
                };
                _:_ => panic!("Invalid answer received");
            }
        }));
        i += 1;
    }

    eprintln!("Blocking...");

    Bastion::block_until_stopped();
}
