use lightproc::prelude::*;

fn main() {
    LightProc::<()>::new().with_future(async move {
        println!("test");
    });
}
