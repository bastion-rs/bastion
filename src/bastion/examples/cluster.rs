use bastion::distributed::*;
use bastion::prelude::*;

use lazy_static::*;

use std::net::ToSocketAddrs;
use std::sync::Arc;
use uuid::Uuid;

use futures_timer::Delay;
use std::time::Duration;

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
/// Spawns node that will assemble local eventually consistent Bastion cluster
///
/// Prologue:
/// This example does the following:
/// * starts MDNS service discovery mechanism and checks nodes throughout this service's lifetime.
/// * discovered nodes are joining to Bastion cluster
/// * sends a String message to every member that joined to cluster and this node knows.
/// * listens incoming messages from other nodes and prints them.
///
/// Bastion's cluster is using fixed size UDP packets to communicate.
/// These can be used to assemble further level of membership and data interchange.
fn main() {
    env_logger::init();

    Bastion::init();

    // Assemble this node's actor
    Bastion::distributed(
        &*CLUSTER_CONFIG,
        |dctx: Arc<DistributedContext>| async move {
            // Assemble outbound action for node
            let outdctx = dctx.clone();
            let _outbound = blocking!(loop {
                outdctx.members().iter().for_each(|m| {
                    let message = format!("PING FROM {}", outdctx.current());
                    outdctx.tell(&m.host_key(), message);
                });

                let _member_msg_wait = Delay::new(Duration::from_secs(1)).await;
            });

            // Assemble inbound action for node
            println!("Started listening...");
            loop {
                let mmsg = dctx.recv().await?.extract();
                let member_msg: String = mmsg.downcast().unwrap();
                println!("Message received: {:?}", member_msg);
            }

            Ok(())
        },
    )
    .expect("Couldn't start cluster node.");

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
