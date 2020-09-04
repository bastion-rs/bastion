// extern crate bastion;
//
// use bastion::prelude::*;
// use std::panic;
//
// fn setup() {
//     Bastion::init();
//     Bastion::start();
// }
//
// fn teardown() {
//     Bastion::stop();
//     Bastion::block_until_stopped();
// }
//
// fn spawn_responders() -> ChildrenRef {
//     Bastion::children(|children: Children| {
//         children.with_exec(move |ctx: BastionContext| async move {
//             msg! { ctx.recv().await?,
//                 msg: &'static str =!> {
//                     if msg == "Hello" {
//                             assert!(signature!().is_sender_identified(), false);
//                             answer!(ctx, "Goodbye").unwrap();
//                     }
//                 };
//                 _: _ => ();
//             }
//
//             msg! { ctx.recv().await?,
//                 msg: &'static str => {
//                     if msg == "Hi again" {
//                         let sign = signature!();
//                         ctx.tell(&sign, "Farewell").unwrap();
//                     }
//                 };
//                 _: _ => ();
//             }
//
//             Ok(())
//         })
//     })
//     .expect("Couldn't create the children group.")
// }
//
// #[test]
// fn answer_and_tell_signatures() {
//     setup();
//     Bastion::spawn(run).unwrap();
//     teardown();
// }
//
// async fn run(ctx: BastionContext) -> Result<(), ()> {
//     let responders = spawn_responders();
//     let responder = &responders.elems()[0];
//     let answer = ctx.ask(&responder.addr(), "Hello").unwrap();
//     let (msg, sign) = answer.await.unwrap().extract();
//     let msg: &str = msg.downcast().unwrap();
//     assert_eq!(msg, "Goodbye");
//
//     let path = sign.path();
//     let elem = path.elem().as_ref().expect("elem is not present");
//     assert!(elem.is_child());
//     ctx.tell(&sign, "Hi again").unwrap();
//
//     let (msg, _) = ctx.recv().await.unwrap().extract();
//     let msg: &str = msg.downcast().unwrap();
//     assert_eq!(msg, "Farewell");
//     Ok(())
// }
//
// // TODO: anonymous signatures Bastion::* methods
