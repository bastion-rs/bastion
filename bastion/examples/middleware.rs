use bastion::prelude::*;
use snap::*;
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::Arc;

///
/// Middleware that compresses streams with snappy with built-in back-pressure
///
/// Prologue:
/// This example starts a TCP server and compresses all requests with Snappy compression
/// using fair distribution of workers.
///
/// Workers capability of processing defines the back-pressure.
/// This back-pressure is most natural one in the Rust environment since
/// it is based on the workload(bottom-to-up) and
/// not based on the source emission(top-to-bottom).
///
/// Try increasing the worker count. Yes!
/// In addition to that try benchmarking with `ab`, `wrk` or `siege`.
fn main() {
    env_logger::init();

    Bastion::init();

    // Workers that process the work.
    let workers = Bastion::children(|children: Children| {
        children
            .with_redundancy(100) // Let's have a pool of an hundred workers.
            .with_exec(move |ctx: BastionContext| {
                async move {
                    println!("Worker started!");

                    // Start receiving work
                    loop {
                        msg! { ctx.recv().await?,
                            stream: TcpStream =!> {
                                let mut stream = stream;
                                let mut data_buf = [0 as u8; 1024];
                                let rb = stream.read(&mut data_buf).unwrap();
                                println!("Received {} bytes", rb);
                                let compressed = Encoder::new().compress_vec(&data_buf).unwrap();
                                let response = escape(&compressed);
                                // println!("Response: {}", response);
                                stream.write(response.as_bytes()).unwrap();
                                answer!(ctx, stream).expect("Couldn't send an answer.");
                            };
                            _: _ => ();
                        }
                    }
                }
            })
    })
    .expect("Couldn't start a new children group.");

    // Get a shadowed sharable reference of workers.
    let workers = Arc::new(workers);

    //
    // Server entrypoint
    Bastion::children(|children: Children| {
        children.with_exec(move |ctx: BastionContext| {
            let workers = workers.clone();
            async move {
                println!("Server is starting!");

                let listener = TcpListener::bind("127.0.0.1:2278").unwrap();

                let mut round_robin = 0;
                for stream in listener.incoming() {
                    // Make a fair distribution to workers
                    round_robin += 1;
                    round_robin %= workers.elems().len();

                    // Distribute tcp streams
                    let _ = ctx
                        .ask(&workers.elems()[round_robin].addr(), stream.unwrap())
                        .unwrap()
                        .await?;
                }

                // Send a signal to system that computation is finished.
                Bastion::stop();

                Ok(())
            }
        })
    })
    .expect("Couldn't start a new children group.");

    Bastion::start();
    Bastion::block_until_stopped();
}

fn escape(bytes: &[u8]) -> String {
    use std::ascii::escape_default;
    bytes
        .iter()
        .flat_map(|&b| escape_default(b))
        .map(|b| b as char)
        .collect()
}
