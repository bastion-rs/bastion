use bastion::prelude::*;
#[cfg(not(target_os = "windows"))]
use futures::io;
#[cfg(target_os = "windows")]
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream, ToSocketAddrs};
use std::sync::atomic::{AtomicUsize, Ordering};

#[cfg(not(target_os = "windows"))]
async fn run(addr: impl ToSocketAddrs) -> io::Result<()> {
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

#[cfg(target_os = "windows")]
async fn run(addr: impl ToSocketAddrs) -> io::Result<()> {
    let listener = std::net::TcpListener::bind(addr).unwrap();
    println!("Listening on {}", listener.local_addr().unwrap());

    // Accept clients in a loop.
    while let Ok(stream) = listener.incoming() {
        println!("Accepted client");
        // Spawn a task that echoes messages from the client back to it.
        spawn(echo(stream));
    }
    Ok(())
}

#[cfg(not(target_os = "windows"))]
async fn echo(stream: Handle<TcpStream>) -> io::Result<()> {
    io::copy(&stream, &mut &stream).await?;
    Ok(())
}

#[cfg(target_os = "windows")]
async fn echo(stream: TcpStream) -> io::Result<()> {
    let mut buf = [0 as u8; 256];
    while let Ok(size) = stream.read(&mut buf) {
        stream.write(&buf[0..size]).unwrap();
    }
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
/// non windows versions use io::copy from nuclei
/// windows versions use a regular stream copy
fn main() {
    env_logger::init();

    Bastion::init();

    let _tcp_servers = Bastion::children(|children: Children| {
        children
            .with_redundancy(TCP_SERVER_COUNT) // Let's have 10 tcp echo servers :)
            .with_exec(move |_ctx: BastionContext| async move {
                println!("Server is starting!");
                let port = TCP_SERVERS.fetch_sub(1, Ordering::SeqCst) + 2000;
                let addr = format!("127.0.0.1:{}", port);

                run(addr);

                Ok(())
            })
    })
    .expect("Couldn't start a new children group.");

    Bastion::start();
    Bastion::block_until_stopped();
}
