use lazy_static::*;
use std::thread;

pub struct LoadBalancer();

impl LoadBalancer {
    pub fn trigger() {
        unimplemented!()
    }
}

#[inline]
pub(crate) fn launch() -> &'static LoadBalancer {
    lazy_static! {
        static ref LOAD_BALANCER: LoadBalancer = {
            thread::Builder::new()
                .name("load-balancer-thread".to_string())
                .spawn(|| {

                })
                .expect("load-balancer couldn't start");

            LoadBalancer()
        };
    }
    &*LOAD_BALANCER
}
