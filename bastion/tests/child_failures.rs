extern crate bastion;

mod helpers;

use bastion::prelude::*;
use helpers::*;
use std::thread;
use std::time::Duration;

fn spawn_responders() -> ChildrenRef {
    Bastion::children(|children: Children| {
        children.with_exec(move |ctx: BastionContext| {
            async move {
                let mut said_hello = false;

                loop {
                    msg! { ctx.recv().await?,
                        msg: &'static str =!> {
                            match msg {
                                "Hello" => {
                                    println!("<<< received ask hello");
                                    if said_hello {
                                        // sleep to not bloat output if test fails
                                        thread::sleep(Duration::from_secs(1));
                                        panic!("Already said hello");
                                    }
                                    answer!(ctx, "Hello").unwrap();
                                    said_hello = true;
                                },
                                _ => (),
                            }
                        };
                        msg: &'static str => {
                            match msg {
                                "Hello" => {
                                    println!("<<< received tell hello");
                                    ctx.tell(&signature!(), "Hello").unwrap();
                                },
                                _ => (),
                            }
                        };
                        _: _ => ();
                    }
                }
            }
        })
    })
    .expect("Couldn't create the children group.")
}

#[test]
fn preserve_mailbox() {
    init_start();
    Bastion::spawn(|ctx: BastionContext| {
        async move {
            let responders = spawn_responders();
            let responder = &responders.elems()[0];
            let answer1 = ctx
                .ask(&responder.addr(), "Hello")
                .expect("Unable to ask 1");
            // this should fail
            let answer2 = ctx
                .ask(&responder.addr(), "Hello")
                .expect("Unable to ask 2");
            // this should be processed by a respawned child
            /*let answer3 = ctx
                .ask(&responder.addr(), "Hello")
                .expect("Unable to ask 3");

            assert!(answer1.await.is_ok());
            assert!(answer2.await.is_err());
            
            let answer3 = answer3.await;
            assert!(answer3.is_ok());

            // the ID should remain the same
            let (msg, sign) = answer3.unwrap().extract();
            assert_eq!(sign.path().id(), responder.id());*/
            
            println!("telling");
            ctx
                .tell(&responder.addr(), "Hello")
                .expect("Unable to tell 4");
            
            println!("awaiting");
            let (msg, sign) = ctx.recv().await?.extract();
            println!("asserting");
            assert_eq!(sign.path().id(), responder.id());
            
            Bastion::stop();

            Ok(())
        }
    })
    .unwrap();

    Bastion::block_until_stopped();
}
