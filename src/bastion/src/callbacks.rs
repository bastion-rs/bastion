use std::fmt::{self, Debug, Formatter};
use std::sync::Arc;

#[derive(Debug, Clone)]
#[allow(dead_code)]
pub(crate) enum CallbackType {
    AfterRestart,
    AfterStop,
    BeforeRestart,
    BeforeStart,
}

#[derive(Default, Clone)]
/// A set of methods that will get called at different states of
/// a [`Supervisor`] or [`Children`] life.
///
/// # Example
///
/// ```rust
/// # use bastion::prelude::*;
/// #
/// # #[cfg(feature = "tokio-runtime")]
/// # #[tokio::main]
/// # async fn main() {
/// #    run();    
/// # }
/// #
/// # #[cfg(not(feature = "tokio-runtime"))]
/// # fn main() {
/// #    run();    
/// # }
/// #
/// # fn run() {
/// # Bastion::init();
/// #
/// Bastion::children(|children| {
///     let callbacks = Callbacks::new()
///         .with_before_start(|| println!("Children group started."))
///         .with_after_stop(|| println!("Children group stopped."));
///
///     children
///         .with_callbacks(callbacks)
///         .with_exec(|ctx| {
///             // -- Children group started.
///             async move {
///                 // ...
///                 # Ok(())
///             }
///             // -- Children group stopped.
///         })
/// }).expect("Couldn't create the children group.");
/// #
/// # Bastion::start();
/// # Bastion::stop();
/// # Bastion::block_until_stopped();
/// # }
/// ```
///
/// [`Supervisor`]: supervisor/struct.Supervisor.html
/// [`Children`]: children/struct.Children.html
pub struct Callbacks {
    before_start: Option<Arc<dyn Fn() + Send + Sync>>,
    before_restart: Option<Arc<dyn Fn() + Send + Sync>>,
    after_restart: Option<Arc<dyn Fn() + Send + Sync>>,
    after_stop: Option<Arc<dyn Fn() + Send + Sync>>,
}

impl Callbacks {
    /// Creates a new instance of `Callbacks` for
    /// [`Supervisor::with_callbacks`] or [`Children::with_callbacks`].
    ///
    /// # Example
    ///
    /// ```rust
    /// # use bastion::prelude::*;
    /// #
    /// # #[cfg(feature = "tokio-runtime")]
    /// # #[tokio::main]
    /// # async fn main() {
    /// #    run();    
    /// # }
    /// #
    /// # #[cfg(not(feature = "tokio-runtime"))]
    /// # fn main() {
    /// #    run();    
    /// # }
    /// #
    /// # fn run() {
    /// # Bastion::init();
    /// #
    /// Bastion::children(|children| {
    ///     let callbacks = Callbacks::new()
    ///         .with_before_start(|| println!("Children group started."))
    ///         .with_after_stop(|| println!("Children group stopped."));
    ///
    ///     children
    ///         .with_callbacks(callbacks)
    ///         .with_exec(|ctx| {
    ///             // -- Children group started.
    ///             async move {
    ///                 // ...
    ///                 # Ok(())
    ///             }
    ///             // -- Children group stopped.
    ///         })
    /// }).expect("Couldn't create the children group.");
    /// #
    /// # Bastion::start();
    /// # Bastion::stop();
    /// # Bastion::block_until_stopped();
    /// # }
    /// ```
    ///
    /// [`Supervisor::with_callbacks`]: supervisor/struct.Supervisor.html#method.with_callbacks
    /// [`Children::with_callbacks`]: children/struct.Children.html#method.with_callbacks
    pub fn new() -> Self {
        Callbacks::default()
    }

    /// Sets the method that will get called before the [`Supervisor`]
    /// or [`Children`] is launched if:
    /// - it was never called before
    /// - or the supervisor of the supervised element using this callback
    ///     (or the system) decided to restart it and it was already
    ///     stopped or killed
    /// - or the supervisor of the supervised element using this callback
    ///     (or the system) decided to restart it and it wasn't already
    ///     stopped or killed but did not have a callback defined using
    ///     [`with_after_restart`]
    ///
    /// # Example
    ///
    /// ```rust
    /// # use bastion::prelude::*;
    /// #
    /// # #[cfg(feature = "tokio-runtime")]
    /// # #[tokio::main]
    /// # async fn main() {
    /// #    run();    
    /// # }
    /// #
    /// # #[cfg(not(feature = "tokio-runtime"))]
    /// # fn main() {
    /// #    run();    
    /// # }
    /// #
    /// # fn run() {
    /// # Bastion::init();
    /// #
    /// # Bastion::supervisor(|supervisor| {
    /// supervisor.children(|children| {
    ///     let callbacks = Callbacks::new()
    ///         .with_before_start(|| println!("Children group started."))
    ///         .with_before_restart(|| println!("Children group restarting."))
    ///         .with_after_restart(|| println!("Children group restarted."))
    ///         .with_after_stop(|| println!("Children group stopped."));
    ///
    ///     children
    ///         .with_exec(|ctx| {
    ///             // -- Children group started.
    ///             async move {
    ///                 // ...
    ///
    ///                 // This will stop the children group...
    ///                 Ok(())
    ///                 // Note that because the children group stopped by itself,
    ///                 // if its supervisor restarts it, its `before_start` callback
    ///                 // will get called and not `after_restart`.
    ///             }
    ///             // -- Children group stopped.
    ///         })
    ///         .with_callbacks(callbacks)
    /// })
    /// # }).unwrap();
    /// #
    /// # Bastion::start();
    /// # Bastion::stop();
    /// # Bastion::block_until_stopped();
    /// # }
    /// ```
    ///
    /// [`Supervisor`]: supervisor/struct.Supervisor.html
    /// [`Children`]: children/struct.Children.html
    /// [`with_after_restart`]: #method.with_after_start
    pub fn with_before_start<C>(mut self, before_start: C) -> Self
    where
        C: Fn() + Send + Sync + 'static,
    {
        let before_start = Arc::new(before_start);
        self.before_start = Some(before_start);
        self
    }

    /// Sets the method that will get called before the [`Supervisor`]
    /// or [`Children`] is reset if:
    /// - the supervisor of the supervised element using this callback
    ///     (or the system) decided to restart it and it wasn't already
    ///     stopped or killed
    ///
    /// Note that if this callback isn't defined but one was defined using
    /// [`with_after_stop`], it will get called instead.
    ///
    /// # Example
    ///
    /// ```rust
    /// # use bastion::prelude::*;
    /// #
    /// # #[cfg(feature = "tokio-runtime")]
    /// # #[tokio::main]
    /// # async fn main() {
    /// #    run();    
    /// # }
    /// #
    /// # #[cfg(not(feature = "tokio-runtime"))]
    /// # fn main() {
    /// #    run();    
    /// # }
    /// #
    /// # fn run() {
    /// # Bastion::init();
    /// #
    /// # Bastion::supervisor(|supervisor| {
    /// supervisor.children(|children| {
    ///     let callbacks = Callbacks::new()
    ///         .with_before_start(|| println!("Children group started."))
    ///         .with_before_restart(|| println!("Children group restarting."))
    ///         .with_after_restart(|| println!("Children group restarted."))
    ///         .with_after_stop(|| println!("Children group stopped."));
    ///
    ///     children
    ///         .with_exec(|ctx| {
    ///             // Once -- Children group started.
    ///             // and then -- Children group restarted.
    ///             async move {
    ///                 // ...
    ///
    ///                 // This will make the children group fault and get
    ///                 // restarted by its supervisor...
    ///                 Err(())
    ///             }
    ///             // -- Children group restarting.
    ///             // Note that if a `before_restart` wasn't specified for
    ///             // this children group, `after_stop` would get called
    ///             // instead.
    ///         })
    ///         .with_callbacks(callbacks)
    /// })
    /// # }).unwrap();
    /// #
    /// # Bastion::start();
    /// # Bastion::stop();
    /// # Bastion::block_until_stopped();
    /// # }
    /// ```
    ///
    /// [`Supervisor`]: supervisor/struct.Supervisor.html
    /// [`Children`]: children/struct.Children.html
    /// [`with_after_stop`]: #method.with_after_stop
    pub fn with_before_restart<C>(mut self, before_restart: C) -> Self
    where
        C: Fn() + Send + Sync + 'static,
    {
        let before_restart = Arc::new(before_restart);
        self.before_restart = Some(before_restart);
        self
    }

    /// Sets the method that will get called before the [`Supervisor`]
    /// or [`Children`] is launched if:
    /// - the supervisor of the supervised element using this callback
    ///     (or the system) decided to restart it and it wasn't already
    ///     stopped or killed
    ///
    /// Note that if this callback isn't defined but one was defined using
    /// [`with_before_start`], it will get called instead.
    ///
    /// # Example
    ///
    /// ```rust
    /// # use bastion::prelude::*;
    /// #
    /// # #[cfg(feature = "tokio-runtime")]
    /// # #[tokio::main]
    /// # async fn main() {
    /// #    run();    
    /// # }
    /// #
    /// # #[cfg(not(feature = "tokio-runtime"))]
    /// # fn main() {
    /// #    run();    
    /// # }
    /// #
    /// # fn run() {
    /// # Bastion::init();
    /// #
    /// # Bastion::supervisor(|supervisor| {
    /// supervisor.children(|children| {
    ///     let callbacks = Callbacks::new()
    ///         .with_before_start(|| println!("Children group started."))
    ///         .with_before_restart(|| println!("Children group restarting."))
    ///         .with_after_restart(|| println!("Children group restarted."))
    ///         .with_after_stop(|| println!("Children group stopped."));
    ///
    ///     children
    ///         .with_exec(|ctx| {
    ///             // Once -- Children group started.
    ///             // and then -- Children group restarted.
    ///             // Note that if a `after_restart` callback wasn't specified
    ///             // for this children group, `before_restart` would get called
    ///             // instead.
    ///             async move {
    ///                 // ...
    ///
    ///                 // This will make the children group fault and get
    ///                 // restarted by its supervisor...
    ///                 Err(())
    ///             }
    ///             // -- Children group restarting.
    ///         })
    ///         .with_callbacks(callbacks)
    /// })
    /// # }).unwrap();
    /// #
    /// # Bastion::start();
    /// # Bastion::stop();
    /// # Bastion::block_until_stopped();
    /// # }
    /// ```
    ///
    /// [`Supervisor`]: supervisor/struct.Supervisor.html
    /// [`Children`]: children/struct.Children.html
    /// [`with_before_start`]: #method.with_before_start
    pub fn with_after_restart<C>(mut self, after_restart: C) -> Self
    where
        C: Fn() + Send + Sync + 'static,
    {
        let after_restart = Arc::new(after_restart);
        self.after_restart = Some(after_restart);
        self
    }

    /// Sets the method that will get called after the [`Supervisor`]
    /// or [`Children`] is stopped or killed if:
    /// - the supervisor of the supervised element using this callback
    ///     (or the system) decided to stop (not restart nor kill) it and
    ///     it wasn't already stopped or killed
    /// - or the supervisor or children group using this callback
    ///     stopped or killed itself or was stopped or killed by a
    ///     reference to it
    /// - or the supervisor of the supervised element using this callback
    ///     (or the system) decided to restart it and it wasn't already
    ///     stopped or killed but did not have a callback defined using
    ///     [`with_before_restart`]
    ///
    /// # Example
    ///
    /// ```rust
    /// # use bastion::prelude::*;
    /// #
    /// # #[cfg(feature = "tokio-runtime")]
    /// # #[tokio::main]
    /// # async fn main() {
    /// #    run();    
    /// # }
    /// #
    /// # #[cfg(not(feature = "tokio-runtime"))]
    /// # fn main() {
    /// #    run();    
    /// # }
    /// #
    /// # fn run() {
    /// # Bastion::init();
    /// #
    /// # Bastion::supervisor(|supervisor| {
    /// supervisor.children(|children| {
    ///     let callbacks = Callbacks::new()
    ///         .with_before_start(|| println!("Children group started."))
    ///         .with_before_restart(|| println!("Children group restarting."))
    ///         .with_after_restart(|| println!("Children group restarted."))
    ///         .with_after_stop(|| println!("Children group stopped."));
    ///
    ///     children
    ///         .with_exec(|ctx| {
    ///             // -- Children group started.
    ///             async move {
    ///                 // ...
    ///
    ///                 // This will stop the children group...
    ///                 Ok(())
    ///             }
    ///             // -- Children group stopped.
    ///             // Note that because the children group stopped by itself,
    ///             // it its supervisor restarts it, its `before_restart` callback
    ///             // will not get called.
    ///         })
    ///         .with_callbacks(callbacks)
    /// })
    /// # }).unwrap();
    /// #
    /// # Bastion::start();
    /// # Bastion::stop();
    /// # Bastion::block_until_stopped();
    /// # }
    /// ```
    ///
    /// [`Supervisor`]: supervisor/struct.Supervisor.html
    /// [`Children`]: children/struct.Children.html
    /// [`with_before_restart`]: #method.with_before_restart
    pub fn with_after_stop<C>(mut self, after_stop: C) -> Self
    where
        C: Fn() + Send + Sync + 'static,
    {
        let after_stop = Arc::new(after_stop);
        self.after_stop = Some(after_stop);
        self
    }

    /// Returns whether a callback was defined using [`with_before_start`].
    ///
    /// # Example
    ///
    /// ```rust
    /// # use bastion::prelude::*;
    /// #
    /// let callbacks = Callbacks::new()
    ///     .with_before_start(|| println!("Children group started."));
    ///
    /// assert!(callbacks.has_before_start());
    /// ```
    ///
    /// [`with_before_start`]: #method.with_before_start
    pub fn has_before_start(&self) -> bool {
        self.before_start.is_some()
    }

    /// Returns whether a callback was defined using [`with_before_restart`].
    ///
    /// # Example
    ///
    /// ```rust
    /// # use bastion::prelude::*;
    /// #
    /// let callbacks = Callbacks::new()
    ///     .with_before_restart(|| println!("Children group restarting."));
    ///
    /// assert!(callbacks.has_before_restart());
    /// ```
    ///
    /// [`with_before_restart`]: #method.with_before_restart
    pub fn has_before_restart(&self) -> bool {
        self.before_restart.is_some()
    }

    /// Returns whether a callback was defined using [`with_after_restart`].
    ///
    /// # Example
    ///
    /// ```rust
    /// # use bastion::prelude::*;
    /// #
    /// let callbacks = Callbacks::new()
    ///     .with_after_restart(|| println!("Children group restarted."));
    ///
    /// assert!(callbacks.has_after_restart());
    /// ```
    ///
    /// [`with_after_restart`]: #method.with_after_restart
    pub fn has_after_restart(&self) -> bool {
        self.after_restart.is_some()
    }

    /// Returns whether a callback was defined using [`with_after_stop`].
    ///
    /// # Example
    ///
    /// ```rust
    /// # use bastion::prelude::*;
    /// #
    /// let callbacks = Callbacks::new()
    ///     .with_after_stop(|| println!("Children group stopped."));
    ///
    /// assert!(callbacks.has_after_stop());
    /// ```
    ///
    /// [`with_after_stop`]: #method.with_after_stop
    pub fn has_after_stop(&self) -> bool {
        self.after_stop.is_some()
    }

    pub(crate) fn before_start(&self) {
        if let Some(before_start) = &self.before_start {
            before_start()
        }
    }

    pub(crate) fn before_restart(&self) {
        if let Some(before_restart) = &self.before_restart {
            before_restart()
        } else {
            self.after_stop()
        }
    }

    pub(crate) fn after_restart(&self) {
        if let Some(after_restart) = &self.after_restart {
            after_restart()
        } else {
            self.before_start()
        }
    }

    pub(crate) fn after_stop(&self) {
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
