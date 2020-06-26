//! A module that exposes the functions used under the hoods from `bastion`s macros: `spawn!`, `run!`
//! and `blocking!`.
pub use lightproc::proc_stack::ProcStack;
use std::future::Future;
use lightproc::recoverable_handle::RecoverableHandle;

/// Spawns a blocking task, which will run on the blocking thread pool,
/// and returns the handle.
///
/// # Example
/// ```
/// # use std::{thread, time};
/// # use bastion::executor::blocking;
/// # fn main() {
/// let task = blocking(async move {
///     thread::sleep(time::Duration::from_millis(3000));
/// });
/// # }
/// ```
pub fn blocking<F, R>(future: F) -> RecoverableHandle<R> where
    F: Future<Output = R> + Send + 'static,
    R: Send + 'static
{
    bastion_executor::blocking::spawn_blocking(future, lightproc::proc_stack::ProcStack::default())
}


