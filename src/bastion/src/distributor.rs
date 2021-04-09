//! `Distributor` is a mechanism that allows you to send messages to children.

use crate::{
    message::{Answer, Message, MessageHandler},
    prelude::{ChildRef, SendError},
    system::{STRING_INTERNER, SYSTEM},
};
use anyhow::Result as AnyResult;
use lasso::Spur;
use std::{
    fmt::Debug,
    sync::mpsc::{channel, Receiver},
};

// Copy is fine here because we're working
// with interned strings here
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
/// The `Distributor` is the main message passing mechanism we will use.
/// it provides methods that will allow us to send messages
/// and add/remove actors to the Distribution list
pub struct Distributor(Spur);

impl Distributor {
    /// Create a new distributor to send messages to
    /// # Example
    ///
    /// ```rust
    /// # use bastion::prelude::*;
    /// #
    /// #
    /// # fn run() {
    /// #
    /// let distributor = Distributor::named("my target group");
    /// // distributor is now ready to use
    /// # }
    /// ```
    pub fn named(name: impl AsRef<str>) -> Self {
        Self(STRING_INTERNER.get_or_intern(name.as_ref()))
    }

    /// Ask a question to a recipient attached to the `Distributor`
    /// and wait for a reply.
    ///
    /// This can be achieved manually using a `MessageHandler` and `ask_one`.
    /// Ask a question to a recipient attached to the `Distributor`
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use bastion::prelude::*;
    /// #
    /// # #[cfg(feature = "tokio-runtime")]
    /// # #[tokio::main]
    /// # async fn main() {
    /// #    run();    
    /// # }
    /// #
    /// # #[cfg(not(feature = "tokio-runtime"))]
    /// # fn main() {
    /// #    run();    
    /// # }
    /// #
    /// # fn run() {
    /// # Bastion::init();
    /// # Bastion::start();
    /// # Bastion::supervisor(|supervisor| {
    /// #    supervisor.children(|children| {
    /// // attach a named distributor to the children
    /// children
    /// #        .with_redundancy(1)
    ///     .with_distributor(Distributor::named("my distributor"))
    ///     .with_exec(|ctx: BastionContext| {
    ///        async move {
    ///            loop {
    ///                // The message handler needs an `on_question` section
    ///                // that matches the `question` you're going to send,
    ///                // and that will reply with the Type the request expects.
    ///                // In our example, we ask a `&str` question, and expect a `bool` reply.                    
    ///                MessageHandler::new(ctx.recv().await?)
    ///                    .on_question(|message: &str, sender| {
    ///                        if message == "is it raining today?" {
    ///                            sender.reply(true).unwrap();
    ///                        }
    ///                    });
    ///            }
    ///            Ok(())
    ///        }
    ///     })
    /// #   })
    /// # });
    ///
    /// let distributor = Distributor::named("my distributor");
    ///
    /// let reply: Result<bool, SendError> = distributor
    ///    .request("is it raining today?")
    ///    .recv()
    ///    .expect("couldn't receive reply"); // Ok(true)
    ///
    /// # Bastion::stop();
    /// # Bastion::block_until_stopped();
    /// # }
    /// ```
    pub fn request<R: Message>(&self, question: impl Message) -> Receiver<Result<R, SendError>> {
        let (sender, receiver) = channel();
        match SYSTEM.dispatcher().ask(*self, question) {
            Ok(response) => {
                spawn!(async move {
                    if let Ok(message) = response.await {
                        let message_to_send = MessageHandler::new(message)
                            .on_tell(|reply: R, _| Ok(reply))
                            .on_fallback(|_, _| {
                                Err(SendError::Other(anyhow::anyhow!(
                                    "received a message with the wrong type"
                                )))
                            });
                        let _ = sender.send(message_to_send);
                    } else {
                        let _ = sender.send(Err(SendError::Other(anyhow::anyhow!(
                            "couldn't receive reply"
                        ))));
                    }
                });
            }
            Err(error) => {
                let _ = sender.send(Err(error));
            }
        };

        receiver
    }

    /// Ask a question to a recipient attached to the `Distributor`
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use bastion::prelude::*;
    /// #
    /// # #[cfg(feature = "tokio-runtime")]
    /// # #[tokio::main]
    /// # async fn main() {
    /// #    run();    
    /// # }
    /// #
    /// # #[cfg(not(feature = "tokio-runtime"))]
    /// # fn main() {
    /// #    run();    
    /// # }
    /// #
    /// # fn run() {
    /// # Bastion::init();
    /// # Bastion::supervisor(|supervisor| {
    /// #    supervisor.children(|children| {
    ///     children
    ///         .with_redundancy(1)
    ///         .with_distributor(Distributor::named("my distributor"))
    ///         .with_exec(|ctx: BastionContext| { // ...
    /// #           async move {
    /// #               loop {
    /// #                   let _: Option<SignedMessage> = ctx.try_recv().await;
    /// #               }
    /// #               Ok(())
    /// #           }
    ///         })
    /// #    })
    /// # });
    /// #
    /// # Bastion::start();
    /// # // Wait until everyone is up
    /// # std::thread::sleep(std::time::Duration::from_secs(1));
    ///
    /// let distributor = Distributor::named("my distributor");
    ///
    /// let answer: Answer = distributor.ask_one("hello?").expect("couldn't send question");
    ///
    /// # Bastion::stop();
    /// # Bastion::block_until_stopped();
    /// # }
    /// ```
    pub fn ask_one(&self, question: impl Message) -> Result<Answer, SendError> {
        SYSTEM.dispatcher().ask(*self, question)
    }

    /// Ask a question to all recipients attached to the `Distributor`
    ///
    /// Requires a `Message` that implements `Clone`. (it will be cloned and passed to each recipient)
    /// # Example
    ///
    /// ```no_run
    /// # use bastion::prelude::*;
    /// #
    /// # #[cfg(feature = "tokio-runtime")]
    /// # #[tokio::main]
    /// # async fn main() {
    /// #    run();    
    /// # }
    /// #
    /// # #[cfg(not(feature = "tokio-runtime"))]
    /// # fn main() {
    /// #    run();    
    /// # }
    /// #
    /// # fn run() {
    /// # Bastion::init();
    /// # Bastion::supervisor(|supervisor| {
    /// #    supervisor.children(|children| {
    /// #    children
    /// #        .with_redundancy(1)
    /// #        .with_distributor(Distributor::named("my distributor"))
    /// #        .with_exec(|ctx: BastionContext| {
    /// #           async move {
    /// #               loop {
    /// #                   let _: Option<SignedMessage> = ctx.try_recv().await;
    /// #               }
    /// #               Ok(())
    /// #           }
    /// #        })
    /// #    })
    /// # });
    /// #
    /// # Bastion::start();
    /// # // Wait until everyone is up
    /// # std::thread::sleep(std::time::Duration::from_secs(1));
    ///
    /// let distributor = Distributor::named("my distributor");
    ///
    /// let answer: Vec<Answer> = distributor.ask_everyone("hello?".to_string()).expect("couldn't send question");
    ///
    /// # Bastion::stop();
    /// # Bastion::block_until_stopped();
    /// # }
    /// ```
    pub fn ask_everyone(&self, question: impl Message + Clone) -> Result<Vec<Answer>, SendError> {
        SYSTEM.dispatcher().ask_everyone(*self, question)
    }

    /// Send a Message to a recipient attached to the `Distributor`
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use bastion::prelude::*;
    /// #
    /// # #[cfg(feature = "tokio-runtime")]
    /// # #[tokio::main]
    /// # async fn main() {
    /// #    run();    
    /// # }
    /// #
    /// # #[cfg(not(feature = "tokio-runtime"))]
    /// # fn main() {
    /// #    run();    
    /// # }
    /// #
    /// # fn run() {
    /// # Bastion::init();
    /// # Bastion::supervisor(|supervisor| {
    /// #    supervisor.children(|children| {
    /// #    children
    /// #        .with_redundancy(1)
    /// #        .with_distributor(Distributor::named("my distributor"))
    /// #        .with_exec(|ctx: BastionContext| {
    /// #           async move {
    /// #               loop {
    /// #                   let _: Option<SignedMessage> = ctx.try_recv().await;
    /// #               }
    /// #               Ok(())
    /// #           }
    /// #        })
    /// #    })
    /// # });
    /// #
    /// # Bastion::start();
    /// # // Wait until everyone is up
    /// # std::thread::sleep(std::time::Duration::from_secs(1));
    ///
    /// let distributor = Distributor::named("my distributor");
    ///
    /// let answer: () = distributor.tell_one("hello?").expect("couldn't send question");
    ///
    /// # Bastion::stop();
    /// # Bastion::block_until_stopped();
    /// # }
    /// ```
    pub fn tell_one(&self, message: impl Message) -> Result<(), SendError> {
        SYSTEM.dispatcher().tell(*self, message)
    }

    /// Send a Message to each recipient attached to the `Distributor`
    ///
    /// Requires a `Message` that implements `Clone`. (it will be cloned and passed to each recipient)
    /// # Example
    ///
    /// ```no_run
    /// # use bastion::prelude::*;
    /// #
    /// # #[cfg(feature = "tokio-runtime")]
    /// # #[tokio::main]
    /// # async fn main() {
    /// #    run();    
    /// # }
    /// #
    /// # #[cfg(not(feature = "tokio-runtime"))]
    /// # fn main() {
    /// #    run();    
    /// # }
    /// #
    /// # fn run() {
    /// # Bastion::init();
    /// # Bastion::supervisor(|supervisor| {
    /// #    supervisor.children(|children| {
    /// #    children
    /// #        .with_redundancy(1)
    /// #        .with_distributor(Distributor::named("my distributor"))
    /// #        .with_exec(|ctx: BastionContext| {
    /// #           async move {
    /// #               loop {
    /// #                   let _: Option<SignedMessage> = ctx.try_recv().await;
    /// #               }
    /// #               Ok(())
    /// #           }
    /// #        })
    /// #    })
    /// # });
    /// #
    /// # Bastion::start();
    /// # // Wait until everyone is up
    /// # std::thread::sleep(std::time::Duration::from_secs(1));
    ///
    /// let distributor = Distributor::named("my distributor");
    ///
    /// let answer: () = distributor.tell_one("hello?").expect("couldn't send question");
    ///
    /// # Bastion::stop();
    /// # Bastion::block_until_stopped();
    /// # }
    /// ```
    pub fn tell_everyone(&self, message: impl Message + Clone) -> Result<Vec<()>, SendError> {
        SYSTEM.dispatcher().tell_everyone(*self, message)
    }

    /// subscribe a `ChildRef` to the named `Distributor`
    ///
    /// ```no_run
    /// # use bastion::prelude::*;
    /// #
    /// # #[cfg(feature = "tokio-runtime")]
    /// # #[tokio::main]
    /// # async fn main() {
    /// #    run();    
    /// # }
    /// #
    /// # #[cfg(not(feature = "tokio-runtime"))]
    /// # fn main() {
    /// #    run();    
    /// # }
    /// #
    /// # fn run() {
    /// # Bastion::init();
    /// # let children =
    /// #    Bastion::children(|children| {
    /// #    children
    /// #        .with_redundancy(1)
    /// #        .with_distributor(Distributor::named("my distributor"))
    /// #        .with_exec(|ctx: BastionContext| {
    /// #           async move {
    /// #               loop {
    /// #                   let _: Option<SignedMessage> = ctx.try_recv().await;
    /// #               }
    /// #               Ok(())
    /// #           }
    /// #        })
    /// # }).unwrap();
    /// #
    /// # Bastion::start();
    /// # // Wait until everyone is up
    /// # std::thread::sleep(std::time::Duration::from_secs(1));
    /// #
    /// let child_ref = children.elems()[0].clone();
    ///
    /// let distributor = Distributor::named("my distributor");
    ///
    /// // child_ref will now be elligible to receive messages dispatched through distributor
    /// distributor.subscribe(child_ref).expect("couldn't subscribe child to distributor");
    ///
    /// # Bastion::stop();
    /// # Bastion::block_until_stopped();
    /// # }
    /// ```
    pub fn subscribe(&self, child_ref: ChildRef) -> AnyResult<()> {
        let global_dispatcher = SYSTEM.dispatcher();
        global_dispatcher.register_recipient(*self, child_ref)
    }

    /// unsubscribe a `ChildRef` to the named `Distributor`
    ///
    /// ```no_run
    /// # use bastion::prelude::*;
    /// #
    /// # #[cfg(feature = "tokio-runtime")]
    /// # #[tokio::main]
    /// # async fn main() {
    /// #    run();    
    /// # }
    /// #
    /// # #[cfg(not(feature = "tokio-runtime"))]
    /// # fn main() {
    /// #    run();    
    /// # }
    /// #
    /// # fn run() {
    /// # Bastion::init();
    /// # let children =
    /// #    Bastion::children(|children| {
    /// #    children
    /// #        .with_redundancy(1)
    /// #        .with_distributor(Distributor::named("my distributor"))
    /// #        .with_exec(|ctx: BastionContext| {
    /// #           async move {
    /// #               loop {
    /// #                   let _: Option<SignedMessage> = ctx.try_recv().await;
    /// #               }
    /// #               Ok(())
    /// #           }
    /// #        })
    /// # }).unwrap();
    /// #
    /// # Bastion::start();
    /// # // Wait until everyone is up
    /// # std::thread::sleep(std::time::Duration::from_secs(1));
    /// #
    /// let child_ref = children.elems()[0].clone();
    ///
    /// let distributor = Distributor::named("my distributor");
    ///
    /// // child_ref will not receive messages dispatched through the distributor anymore
    /// distributor.unsubscribe(child_ref).expect("couldn't unsubscribe child to distributor");
    ///
    /// # Bastion::stop();
    /// # Bastion::block_until_stopped();
    /// # }
    /// ```
    pub fn unsubscribe(&self, child_ref: ChildRef) -> AnyResult<()> {
        let global_dispatcher = SYSTEM.dispatcher();
        global_dispatcher.remove_recipient(&vec![*self], child_ref)
    }

    pub(crate) fn interned(&self) -> &Spur {
        &self.0
    }
}
