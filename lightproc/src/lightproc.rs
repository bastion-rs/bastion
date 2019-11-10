//!
//! Lightweight process implementation which enables users
//! to create either panic recoverable process or
//! ordinary process.
//!
//! Lightweight processes needs a stack to use their lifecycle
//! operations like `before_start`, `after_complete` and more...
//!
//! # Example Usage
//!
//! ```rust
//! use lightproc::prelude::*;
//!
//! // ... future that does work
//! let future = async {
//!     println!("Doing some work");
//! };
//!
//! // ... basic schedule function with no waker logic
//! fn schedule_function(proc: LightProc) {;}
//!
//! // ... process stack with a lifecycle callback
//! let proc_stack =
//!     ProcStack::default()
//!         .with_after_panic(|| {
//!             println!("After panic started!");
//!         });
//!
//! // ... creating a recoverable process
//! let panic_recoverable = LightProc::recoverable(
//!     future,
//!     schedule_function,
//!     proc_stack
//! );
//! ```

use crate::proc_data::ProcData;
use crate::proc_ext::ProcFutureExt;
use crate::proc_handle::ProcHandle;
use crate::proc_stack::*;
use crate::raw_proc::RawProc;
use crate::recoverable_handle::RecoverableHandle;
use std::fmt::{self, Debug, Formatter};
use std::future::Future;
use std::marker::PhantomData;
use std::mem;
use std::panic::AssertUnwindSafe;
use std::ptr::NonNull;

/// Struct to create and operate lightweight processes
pub struct LightProc {
    /// A pointer to the heap-allocated proc.
    pub(crate) raw_proc: NonNull<()>,
}

unsafe impl Send for LightProc {}
unsafe impl Sync for LightProc {}

impl LightProc {
    ///
    /// Creates a recoverable process which will signal occurred
    /// panic back to the poller.
    ///
    /// # Example
    /// ```rust
    /// # use lightproc::prelude::*;
    /// #
    /// # // ... future that does work
    /// # let future = async {
    /// #     println!("Doing some work");
    /// # };
    /// #
    /// # // ... basic schedule function with no waker logic
    /// # fn schedule_function(proc: LightProc) {;}
    /// #
    /// # // ... process stack with a lifecycle callback
    /// # let proc_stack =
    /// #     ProcStack::default()
    /// #         .with_after_panic(|| {
    /// #             println!("After panic started!");
    /// #         });
    /// #
    /// // ... creating a recoverable process
    /// let panic_recoverable = LightProc::recoverable(
    ///     future,
    ///     schedule_function,
    ///     proc_stack
    /// );
    /// ```
    pub fn recoverable<F, R, S>(
        future: F,
        schedule: S,
        stack: ProcStack,
    ) -> (LightProc, RecoverableHandle<R>)
    where
        F: Future<Output = R> + Send + 'static,
        R: Send + 'static,
        S: Fn(LightProc) + Send + Sync + 'static,
    {
        let recovery_future = AssertUnwindSafe(future).catch_unwind();
        let (proc, handle) = Self::build(recovery_future, schedule, stack);
        (proc, RecoverableHandle(handle))
    }

    ///
    /// Creates a standard process which will stop it's execution on occurrence of panic.
    ///
    /// # Example
    /// ```rust
    /// # use lightproc::prelude::*;
    /// #
    /// # // ... future that does work
    /// # let future = async {
    /// #     println!("Doing some work");
    /// # };
    /// #
    /// # // ... basic schedule function with no waker logic
    /// # fn schedule_function(proc: LightProc) {;}
    /// #
    /// # // ... process stack with a lifecycle callback
    /// # let proc_stack =
    /// #     ProcStack::default()
    /// #         .with_after_panic(|| {
    /// #             println!("After panic started!");
    /// #         });
    /// #
    /// // ... creating a standard process
    /// let standard = LightProc::build(
    ///     future,
    ///     schedule_function,
    ///     proc_stack
    /// );
    /// ```
    pub fn build<F, R, S>(future: F, schedule: S, stack: ProcStack) -> (LightProc, ProcHandle<R>)
    where
        F: Future<Output = R> + Send + 'static,
        R: Send + 'static,
        S: Fn(LightProc) + Send + Sync + 'static,
    {
        let raw_proc = RawProc::allocate(stack, future, schedule);
        let proc = LightProc { raw_proc: raw_proc };
        let handle = ProcHandle {
            raw_proc: raw_proc,
            _marker: PhantomData,
        };
        (proc, handle)
    }

    ///
    /// Schedule the lightweight process with passed `schedule` function at the build time.
    pub fn schedule(self) {
        let ptr = self.raw_proc.as_ptr();
        let pdata = ptr as *const ProcData;
        mem::forget(self);

        unsafe {
            ((*pdata).vtable.schedule)(ptr);
        }
    }

    ///
    /// Schedule the lightproc for runnning on the thread.
    pub fn run(self) {
        let ptr = self.raw_proc.as_ptr();
        let pdata = ptr as *const ProcData;
        mem::forget(self);

        unsafe {
            ((*pdata).vtable.run)(ptr);
        }
    }

    ///
    /// Cancel polling the lightproc's inner future, thus cancelling th proc itself.
    pub fn cancel(&self) {
        let ptr = self.raw_proc.as_ptr();
        let pdata = ptr as *const ProcData;

        unsafe {
            (*pdata).cancel();
        }
    }

    ///
    /// Gives a reference to given [ProcStack] when building the light proc.
    pub fn stack(&self) -> &ProcStack {
        let offset = ProcData::offset_stack();
        let ptr = self.raw_proc.as_ptr();

        unsafe {
            let raw = (ptr as *mut u8).add(offset) as *const ProcStack;
            &*raw
        }
    }
}

impl Debug for LightProc {
    fn fmt(&self, fmt: &mut Formatter) -> fmt::Result {
        let ptr = self.raw_proc.as_ptr();
        let pdata = ptr as *const ProcData;

        fmt.debug_struct("LightProc")
            .field("pdata", unsafe { &(*pdata) })
            .field("stack", self.stack())
            .finish()
    }
}

impl Drop for LightProc {
    fn drop(&mut self) {
        let ptr = self.raw_proc.as_ptr();
        let pdata = ptr as *const ProcData;

        unsafe {
            // Cancel the proc.
            (*pdata).cancel();

            // Drop the future.
            ((*pdata).vtable.drop_future)(ptr);

            // Drop the proc reference.
            ((*pdata).vtable.decrement)(ptr);
        }
    }
}
