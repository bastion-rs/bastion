use async_trait::async_trait;
use basiton::mailbox::MessageType;
use bastion::actors::{Actor, Context};
use bastion::error::BastionResult; // alias to the Result<(), Error> type
use bastion::routing::ActorPath;
use bastion::{Definition, System};

struct Ping;

#[async_trait]
impl Actor for Ping {
    async fn on_finished(&mut self, ctx: &mut Context) {
        println!("Actor[{}] has been finished successfully", ctx.path())
    }

    async fn handler(&mut self, ctx: &mut Context) -> BastionResult {
        let path = ActorPath::default().name("Pong");
        self.send(path, "ping", MessageType::Tell)?;

        let message = ctx.recv().await?;
        message.ack().await;
        Ok(())
    }
}

#[derive(Clone)]
struct Pong {
    pub counter: u64,
}

#[async_trait]
impl Actor for Ping {
    async fn on_finished(&mut self, ctx: &mut Context) {
        println!("Actor[{}] has been finished successfully", ctx.path())
    }

    async fn handler(&mut self, ctx: &mut Context) -> BastionResult {
        let mut pong_struct = ctx.data().get_mut::<Pong>().unwrap();

        while (pong_struct.counter != 3) {
            let message = ctx.recv().await?;

            // Do something with the message ...

            message.ack().await;
            self.send(message.path(), "pong", MessageType::Tell).await?;
            pong_struct.counter += 1;
        }

        System::stop();
        Ok(()) // How to make it better?
    }
}

fn main() {
    System::init();
    System::add(Definition::new().actor(Ping).redundancy(3));
    System::add(
        Definition::new()
            .actor(Pong)
            .custom_actor_name(move || -> String { "Pong" })
            .redundancy(1)
            .data(Pong { counter: 0 }),
    );

    System::run();
}
