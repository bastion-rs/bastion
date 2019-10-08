///
/// Basic trampoline code for doing debouncing for supervision restarts.
///
pub enum Tramp<R> {
    Traverse(R),
    Complete(R),
}

///
/// Execution implementation for the debouncer
///
impl<R> Tramp<R> {
    pub fn execute<F>(mut self, f: F) -> R
    where
        F: Fn(R) -> Tramp<R>,
    {
        loop {
            match self {
                Tramp::Traverse(value) => {
                    self = f(value);
                }
                Tramp::Complete(value) => {
                    return value;
                }
            }
        }
    }
}
