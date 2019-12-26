extern crate bastion;

use bastion::prelude::*;

pub fn init_start() {
    env_logger::init();
    Bastion::init();
    Bastion::start();
}
