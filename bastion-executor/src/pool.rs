use crate::distributor::Distributor;
use lazy_static::*;

pub struct Pool {}

#[inline]
pub fn get() -> &'static Pool {
    lazy_static! {
        static ref POOL: Pool = {
            let distributor = Distributor::new();

            distributor.assign(|| {
                println!("1,2,3");
            });

            Pool {}
        };
    }
    &*POOL
}
