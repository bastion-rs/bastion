/// Set if the proc is scheduled for running.
///
/// A proc is considered to be scheduled whenever its `LightProc` reference exists. It is in scheduled
/// state at the moment of creation and when it gets unpaused either by its `ProcHandle` or woken
/// by a `Waker`.
///
/// This flag can't be set when the proc is completed. However, it can be set while the proc is
/// running, in which case it will be rescheduled as soon as polling finishes.
pub(crate) const SCHEDULED: usize = 1;

/// Set if the proc is running.
///
/// A proc is running state while its future is being polled.
///
/// This flag can't be set when the proc is completed. However, it can be in scheduled state while
/// it is running, in which case it will be rescheduled when it stops being polled.
pub(crate) const RUNNING: usize = 1 << 1;

/// Set if the proc has been completed.
///
/// This flag is set when polling returns `Poll::Ready`. The output of the future is then stored
/// inside the proc until it becomes stopped. In fact, `ProcHandle` picks the output up by marking
/// the proc as stopped.
///
/// This flag can't be set when the proc is scheduled or completed.
pub(crate) const COMPLETED: usize = 1 << 2;

/// Set if the proc is closed.
///
/// If a proc is closed, that means its either cancelled or its output has been consumed by the
/// `ProcHandle`. A proc becomes closed when:
///
/// 1. It gets cancelled by `LightProc::cancel()` or `ProcHandle::cancel()`.
/// 2. Its output is awaited by the `ProcHandle`.
/// 3. It panics while polling the future.
/// 4. It is completed and the `ProcHandle` is dropped.
pub(crate) const CLOSED: usize = 1 << 3;

/// Set if the `ProcHandle` still exists.
///
/// The `ProcHandle` is a special case in that it is only tracked by this flag, while all other
/// proc references (`LightProc` and `Waker`s) are tracked by the reference count.
pub(crate) const HANDLE: usize = 1 << 4;

/// Set if the `ProcHandle` is awaiting the output.
///
/// This flag is set while there is a registered awaiter of type `Waker` inside the proc. When the
/// proc gets closed or completed, we need to wake the awaiter. This flag can be used as a fast
/// check that tells us if we need to wake anyone without acquiring the lock inside the proc.
pub(crate) const AWAITER: usize = 1 << 5;

/// Set if the awaiter is locked.
///
/// This lock is acquired before a new awaiter is registered or the existing one is woken.
pub(crate) const LOCKED: usize = 1 << 6;

/// A single reference.
///
/// The lower bits in the state contain various flags representing the proc state, while the upper
/// bits contain the reference count. The value of `REFERENCE` represents a single reference in the
/// total reference count.
///
/// Note that the reference counter only tracks the `LightProc` and `Waker`s. The `ProcHandle` is
/// tracked separately by the `HANDLE` flag.
pub(crate) const REFERENCE: usize = 1 << 7;
