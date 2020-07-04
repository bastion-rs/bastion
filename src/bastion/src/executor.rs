//! A module that exposes the functions used under the hoods from `bastion`s macros: `spawn!`, `run!`
//! and `blocking!`.
pub use lightproc::proc_stack::ProcStack;
use lightproc::recoverable_handle::RecoverableHandle;
use std::future::Future;

/// Spawns a blocking task, which will run on the blocking thread pool,
/// and returns the handle.
///
/// # Example
/// ```
/// # use std::{thread, time};
/// use bastion::executor::blocking;
/// let task = blocking(async move {
///     thread::sleep(time::Duration::from_millis(3000));
/// });
/// ```
pub fn blocking<F, R>(future: F) -> RecoverableHandle<R>
where
    F: Future<Output = R> + Send + 'static,
    R: Send + 'static,
{
    bastion_executor::blocking::spawn_blocking(future, lightproc::proc_stack::ProcStack::default())
}

/// Block the current thread until passed
/// future is resolved with an output (including the panic).
///
/// # Example
/// ```
/// # use bastion::prelude::*;
/// use bastion::executor::run;
/// let future1 = async move {
///     123
/// };
///
/// run(async move {
///     let result = future1.await;
///     assert_eq!(result, 123);
/// });
///
/// let future2 = async move {
///     10 / 2
/// };
///
/// let result = run(future2);
/// assert_eq!(result, 5);
/// ```
pub fn run<F, T>(future: F) -> T
where
    F: Future<Output = T>,
{
    bastion_executor::run::run(future, lightproc::proc_stack::ProcStack::default())
}

/// Spawn a given future onto the executor from the global level.
///
/// # Example
/// ```
/// # use bastion::prelude::*;
/// use bastion::executor::{spawn, run};
/// let handle = spawn(async {
///     panic!("test");
/// });
/// run(handle);
/// ```
pub fn spawn<F, T>(future: F) -> RecoverableHandle<T>
where
    F: Future<Output = T> + Send + 'static,
    T: Send + 'static,
{
    bastion_executor::pool::spawn(future, lightproc::proc_stack::ProcStack::default())
}
