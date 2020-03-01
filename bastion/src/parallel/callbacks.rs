use std::fmt::{self, Debug, Formatter};
use std::sync::{Arc, Mutex};
use std::boxed::Box;
use std::io;

pub type CallbackFunc = dyn FnMut() -> io::Result<()> + Send + Sync + 'static;
pub type SafeCallbackFunc = Arc<Mutex<Box<CallbackFunc>>>;

#[derive(Default)]
pub struct ProcessCallbacks {
    pub before_start: Option<SafeCallbackFunc>,
    pub before_restart: Option<SafeCallbackFunc>,
    pub after_restart: Option<SafeCallbackFunc>,
    pub after_stop: Option<SafeCallbackFunc>,
}

impl Debug for ProcessCallbacks {
    fn fmt(&self, fmt: &mut Formatter) -> fmt::Result {
        fmt.debug_struct("ProcessCallbacks")
            .field("before_start", &self.before_start.is_some())
            .field("before_restart", &self.before_start.is_some())
            .field("after_restart", &self.before_start.is_some())
            .field("after_stop", &self.before_start.is_some())
            .finish()
    }
}

