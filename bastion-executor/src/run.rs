use super::worker;
use crossbeam_utils::sync::Parker;
use lightproc::proc_stack::ProcStack;
use std::cell::Cell;
use std::cell::UnsafeCell;
use std::future::Future;
use std::mem::ManuallyDrop;
use std::pin::Pin;
use std::sync::Arc;
use std::task::{Context, Poll, RawWaker, RawWakerVTable, Waker};
use std::{mem, panic};

pub fn run<F, T>(future: F, stack: ProcStack) -> T
where
    F: Future<Output = T>,
{
    unsafe {
        // A place on the stack where the result will be stored.
        let out = &mut UnsafeCell::new(None);

        // Wrap the future into one that stores the result into `out`.
        let future = {
            let out = out.get();

            async move {
                *out = Some(future.await);
            }
        };

        // Pin the future onto the stack.
        pin_utils::pin_mut!(future);

        // Transmute the future into one that is futurestatic.
        let future = mem::transmute::<
            Pin<&'_ mut dyn Future<Output = ()>>,
            Pin<&'static mut dyn Future<Output = ()>>,
        >(future);

        // Block on the future and and wait for it to complete.
        worker::set_stack(&stack, || block(future));

        // Take out the result.
        match (*out.get()).take() {
            Some(v) => v,
            _ => unimplemented!(),
        }
    }
}

fn block<F, T>(f: F) -> T
where
    F: Future<Output = T>,
{
    thread_local! {
        // May hold a pre-allocated parker that can be reused for efficiency.
        //
        // Note that each invocation of `block` needs its own parker. In particular, if `block`
        // recursively calls itself, we must make sure that each recursive call uses a distinct
        // parker instance.
        static CACHE: Cell<Option<Arc<Parker>>> = Cell::new(None);
    }

    pin_utils::pin_mut!(f);

    CACHE.with(|cache| {
        // Reuse a cached parker or create a new one for this invocation of `block`.
        let arc_parker: Arc<Parker> = cache.take().unwrap_or_else(|| Arc::new(Parker::new()));

        let ptr = (&*arc_parker as *const Parker) as *const ();
        let vt = vtable();

        let waker = unsafe { ManuallyDrop::new(Waker::from_raw(RawWaker::new(ptr, vt))) };
        let cx = &mut Context::from_waker(&waker);

        loop {
            if let Poll::Ready(t) = f.as_mut().poll(cx) {
                // Save the parker for the next invocation of `block`.
                cache.set(Some(arc_parker));
                return t;
            }
            arc_parker.park();
        }
    })
}

fn vtable() -> &'static RawWakerVTable {
    unsafe fn clone_raw(ptr: *const ()) -> RawWaker {
        #![allow(clippy::redundant_clone)]
        let arc = ManuallyDrop::new(Arc::from_raw(ptr as *const Parker));
        mem::forget(arc.clone());
        RawWaker::new(ptr, vtable())
    }

    unsafe fn wake_raw(ptr: *const ()) {
        let arc = Arc::from_raw(ptr as *const Parker);
        arc.unparker().unpark();
    }

    unsafe fn wake_by_ref_raw(ptr: *const ()) {
        let arc = ManuallyDrop::new(Arc::from_raw(ptr as *const Parker));
        arc.unparker().unpark();
    }

    unsafe fn drop_raw(ptr: *const ()) {
        drop(Arc::from_raw(ptr as *const Parker))
    }

    &RawWakerVTable::new(clone_raw, wake_raw, wake_by_ref_raw, drop_raw)
}
