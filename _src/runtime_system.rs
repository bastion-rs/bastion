use crate::runtime_manager::FaultRecovery;
use crate::supervisor::Supervisor;
use ego_tree::Tree;
use parking_lot::Mutex;
use std::any::Any;
use std::sync::Arc;
use tokio::runtime::{Builder, Runtime};

pub struct RuntimeSystem {
    pub runtime: Runtime,
    pub registry: Arc<Mutex<Tree<Supervisor>>>,
}

impl RuntimeSystem {
    pub fn start() -> Self {
        let runtime: Runtime = Builder::new()
            .panic_handler(|err| RuntimeSystem::panic_dispatcher(err))
            .before_stop(|| {
                debug!("System is stopping...");
            })
            .build()
            .unwrap(); // Builder panic isn't a problem since we haven't started.

        let root_supervisor = Supervisor::default().props("root".into(), "root".into());

        let tree = ego_tree::Tree::new(root_supervisor);

        let registry_tree = Arc::new(Mutex::new(tree));

        RuntimeSystem {
            runtime,
            registry: registry_tree,
        }
    }
}

impl FaultRecovery for RuntimeSystem {
    fn panic_dispatcher(_failure: Box<dyn Any + Send>) {}
}
