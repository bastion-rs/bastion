use lazy_static::*;
use crate::distributor::Distributor;

pub struct Pool {

}

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
