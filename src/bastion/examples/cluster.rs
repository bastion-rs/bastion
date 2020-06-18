use bastion::prelude::*;
use std::sync::Arc;
use bastion::distributed::*;
use futures::{select, FutureExt};
use std::net::ToSocketAddrs;
use uuid::Uuid;
use pin_utils::pin_mut;
use futures::{pending, join};
use lazy_static::*;

use std::time::Duration;
use futures_timer::Delay;

use artillery_core::service_discovery::mdns::prelude::*;

lazy_static! {
    static ref CLUSTER_CONFIG: ArtilleryAPClusterConfig = {
        // Let's find a broadcast port
        let port = get_port();

        ArtilleryAPClusterConfig {
            app_name: String::from("artillery-ap"),
            node_id: Uuid::new_v4(),
            sd_config: {
                let mut config = MDNSServiceDiscoveryConfig::default();
                config.local_service_addr.set_port(port);
                config
            },
            cluster_config: {
                let listen_addr = format!("127.0.0.1:{}", port);

                ClusterConfig {
                    listen_addr: (&listen_addr as &str)
                        .to_socket_addrs()
                        .unwrap()
                        .next()
                        .unwrap(),
                    ..Default::default()
                }
            },
        }
    };
}


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

    // Assemble this node's actor
    Bastion::distributed(&*CLUSTER_CONFIG, |dctx: Arc<DistributedContext>| async move {
        // Assemble outbound action for node
        let outdctx = dctx.clone();
        let outbound = blocking!(
            loop {
                outdctx.members()
                    .iter()
                    .for_each(|m| {
                        let message = format!("PING FROM {}", outdctx.current());
                        outdctx.tell(&m.host_key(), message);
                    });

                let member_msg_wait = Delay::new(Duration::from_secs(1)).await;
            }
        );

        // Assemble inbound action for node
        println!("Started listening...");
        loop {
            let mmsg = dctx.recv().await?.extract();
            let member_msg: String = mmsg.downcast().unwrap();
            println!("Message received: {:?}", member_msg);
        }

        Ok(())
    }).expect("Couldn't start cluster node.");

    Bastion::start();
    Bastion::block_until_stopped();
}


fn get_port() -> u16 {
    use rand::{thread_rng, Rng};

    let mut rng = thread_rng();
    let port: u16 = rng.gen();
    if port > 1025 && port < 65535 {
        port
    } else {
        get_port()
    }
}
