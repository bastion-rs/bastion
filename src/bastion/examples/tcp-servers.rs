use bastion::prelude::*;
use futures::io;
use std::net::{TcpListener, TcpStream};
use std::sync::atomic::{AtomicUsize, Ordering};

async fn echo(stream: Handle<TcpStream>) -> io::Result<()> {
    io::copy(&stream, &mut &stream).await?;
    Ok(())
}

const TCP_SERVER_COUNT: usize = 10;
static TCP_SERVERS: AtomicUsize = AtomicUsize::new(TCP_SERVER_COUNT);

///
/// 10 async tcp servers
///
/// Prologue:
///
/// This example demonstrates using 10 parallel tcp servers

fn main() {
    env_logger::init();

    Bastion::init();

    let _tcp_servers = Bastion::children(|children: Children| {
        children
            .with_redundancy(TCP_SERVER_COUNT) // Let's have 40 tcp echo servers :)
            .with_exec(move |_ctx: BastionContext| {
                async move {
                    println!("Server is starting!");
                    let port = TCP_SERVERS.fetch_sub(1, Ordering::SeqCst) + 2000;
                    let addr = format!("127.0.0.1:{}", port);

                    let listener = Handle::<TcpListener>::bind(addr).unwrap();
                    println!("Listening on {}", listener.get_ref().local_addr().unwrap());

                    // Accept clients in a loop.
                    loop {
                        let (stream, peer_addr) = listener.accept().await.unwrap();
                        println!("Accepted client: {}", peer_addr);

                        // Spawn a task that echoes messages from the client back to it.
                        spawn(echo(stream));
                    }

                    Ok(())
                }
            })
    })
    .expect("Couldn't start a new children group.");

    Bastion::start();
    Bastion::block_until_stopped();
}
