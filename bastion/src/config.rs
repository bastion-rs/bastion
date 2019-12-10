#[derive(Default, Debug, Clone)]
/// The configuration that should be used to initialize the
/// system using [`Bastion::init_with`].
///
/// The default behaviors are the following:
/// - All backtraces are shown (see [`Config::show_backtraces`]).
///
/// # Example
///
/// ```rust
/// use bastion::prelude::*;
///
/// fn main() {
///     let config = Config::new().show_backtraces();
///
///     Bastion::init_with(config);
///
///     // You can now use bastion...
///     #
///     # Bastion::start();
///     # Bastion::stop();
///     # Bastion::block_until_stopped();
/// }
/// ```
///
/// [`Bastion::init_with`]: /struct.Bastion.html#method.init_with
/// [`Config::show_backtraces`]: /struct.Config.html#method.show_backtraces
pub struct Config {
    backtraces: Backtraces,
}

#[derive(PartialEq, Eq, Debug, Clone)]
pub(crate) enum Backtraces {
    /// Shows all backtraces, like an application without
    /// Bastion would.
    Show,
    // TODO: Catch,
    /// Hides all backtraces.
    Hide,
}

impl Config {
    /// Creates a new configuration with the following default
    /// behaviors:
    /// - All backtraces are shown (see [`Config::show_backtraces`]).
    ///
    /// [`Config::show_backtraces`]: /struct.Config.html#method.show_backtraces
    pub fn new() -> Self {
        Config::default()
    }

    /// Makes Bastion show all backtraces, like an application
    /// without it would. This can be useful when trying to
    /// debug children panicking.
    ///
    /// Note that this is the default behavior.
    ///
    /// # Example
    ///
    /// ```rust
    /// use bastion::prelude::*;
    ///
    /// fn main() {
    ///     let config = Config::new().show_backtraces();
    ///
    ///     Bastion::init_with(config);
    ///
    ///     // You can now use bastion and it will show you the
    ///     // backtraces of panics...
    ///     #
    ///     # Bastion::start();
    ///     # Bastion::stop();
    ///     # Bastion::block_until_stopped();
    /// }
    /// ```
    pub fn show_backtraces(mut self) -> Self {
        self.backtraces = Backtraces::show();
        self
    }

    /// Makes Bastion hide all backtraces.
    ///
    /// Note that the default behavior is to show all backtraces
    /// (see [`Config::show_backtraces`]).
    ///
    /// # Example
    ///
    /// ```rust
    /// use bastion::prelude::*;
    ///
    /// fn main() {
    ///     let config = Config::new().hide_backtraces();
    ///
    ///     Bastion::init_with(config);
    ///
    ///     // You can now use bastion and no panic backtraces
    ///     // will be shown...
    ///     #
    ///     # Bastion::start();
    ///     # Bastion::stop();
    ///     # Bastion::block_until_stopped();
    /// }
    /// ```
    ///
    /// [`Config::show_backtraces`]: /struct.Config.html#method.show_backtraces
    pub fn hide_backtraces(mut self) -> Self {
        self.backtraces = Backtraces::hide();
        self
    }

    pub(crate) fn backtraces(&self) -> &Backtraces {
        &self.backtraces
    }
}

impl Backtraces {
    fn show() -> Self {
        Backtraces::Show
    }

    fn hide() -> Self {
        Backtraces::Hide
    }

    pub(crate) fn is_hide(&self) -> bool {
        self == &Backtraces::Hide
    }
}

impl Default for Backtraces {
    fn default() -> Self {
        Backtraces::Show
    }
}
