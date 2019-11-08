use std::fmt::{self, Debug, Formatter};
use std::sync::Arc;

pub trait BeforeStart: Fn() + Send + Sync + 'static {}

impl<T> BeforeStart for T where T: Fn() + Send + Sync + 'static {}

pub trait BeforeRestart: Fn() + Send + Sync + 'static {}

impl<T> BeforeRestart for T where T: Fn() + Send + Sync + 'static {}

pub trait AfterRestart: Fn() + Send + Sync + 'static {}

impl<T> AfterRestart for T where T: Fn() + Send + Sync + 'static {}

pub trait AfterStop: Fn() + Send + Sync + 'static {}

impl<T> AfterStop for T where T: Fn() + Send + Sync + 'static {}

#[derive(Default)]
pub struct Callbacks {
    before_start: Option<Arc<dyn BeforeStart>>,
    before_restart: Option<Arc<dyn BeforeRestart>>,
    after_restart: Option<Arc<dyn AfterRestart>>,
    after_stop: Option<Arc<dyn AfterStop>>,
}

impl Callbacks {
    pub fn new() -> Self {
        Callbacks::default()
    }

    pub fn with_before_start<C: BeforeStart>(mut self, before_start: C) -> Self {
        let before_start = Arc::new(before_start);
        self.before_start = Some(before_start);
        self
    }

    pub fn with_before_restart<C: BeforeRestart>(mut self, before_restart: C) -> Self {
        let before_restart = Arc::new(before_restart);
        self.before_restart = Some(before_restart);
        self
    }

    pub fn with_after_restart<C: AfterRestart>(mut self, after_restart: C) -> Self {
        let after_restart = Arc::new(after_restart);
        self.after_restart = Some(after_restart);
        self
    }

    pub fn with_after_stop<C: AfterStop>(mut self, after_stop: C) -> Self {
        let after_stop = Arc::new(after_stop);
        self.after_stop = Some(after_stop);
        self
    }

    pub fn before_start(&self) -> Option<&dyn BeforeStart> {
        if let Some(before_start) = &self.before_start {
            Some(&**before_start)
        } else {
            None
        }
    }

    pub fn before_restart(&self) -> Option<&dyn BeforeRestart> {
        if let Some(before_restart) = &self.before_restart {
            Some(&**before_restart)
        } else {
            None
        }
    }

    pub fn after_restart(&self) -> Option<&dyn AfterRestart> {
        if let Some(after_restart) = &self.after_restart {
            Some(&**after_restart)
        } else {
            None
        }
    }

    pub fn after_stop(&self) -> Option<&dyn AfterStop> {
        if let Some(after_stop) = &self.after_stop {
            Some(&**after_stop)
        } else {
            None
        }
    }

    pub(crate) fn call_before_start(&self) {
        if let Some(before_start) = &self.before_start {
            before_start()
        }
    }

    pub(crate) fn call_before_restart(&self) {
        if let Some(before_restart) = &self.before_restart {
            before_restart()
        } else {
            self.call_after_stop()
        }
    }

    pub(crate) fn call_after_restart(&self) {
        if let Some(after_restart) = &self.after_restart {
            after_restart()
        } else {
            self.call_before_start()
        }
    }

    pub(crate) fn call_after_stop(&self) {
        if let Some(after_stop) = &self.after_stop {
            after_stop()
        }
    }
}

impl Debug for Callbacks {
    fn fmt(&self, fmt: &mut Formatter) -> fmt::Result {
        fmt.debug_struct("Callbacks")
            .field("before_start", &self.before_start.is_some())
            .field("before_restart", &self.before_start.is_some())
            .field("after_restart", &self.before_start.is_some())
            .field("after_stop", &self.before_start.is_some())
            .finish()
    }
}
