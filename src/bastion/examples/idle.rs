use bastion::prelude::*;
use tracing::Level;

fn main() {
    let subscriber = tracing_subscriber::fmt()
        // all spans/events with a level higher than INFO
        // will be written to stdout.
        .with_max_level(Level::INFO)
        // completes the builder and sets the constructed `Subscriber` as the default.
        .finish();
    tracing::subscriber::set_global_default(subscriber).unwrap();
    let config = Config::new().hide_backtraces();
    Bastion::init_with(config);
    Bastion::start();

    run!(async {
        tracing::error!("ok");
    });
    Bastion::block_until_stopped();
}
