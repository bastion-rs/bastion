use std::sync::{Arc, Mutex};
use std::collections::HashMap;
use std::ffi::OsStr;
use std::ffi::OsString;
use std::process::Stdio;
use std::io;
use super::callbacks::*;

#[derive(Debug)]
pub struct ProcessData {
    pub(crate) callbacks: ProcessCallbacks,
    pub(crate) envs: HashMap<OsString, OsString>,
}

impl Default for ProcessData {
    fn default() -> Self {
        Self {
            callbacks: ProcessCallbacks::default(),
            envs: std::env::vars_os().collect(),
        }
    }
}


#[derive(Debug, Default)]
pub struct Builder {
    pub(crate) stdin: Option<Stdio>,
    pub(crate) stdout: Option<Stdio>,
    pub(crate) stderr: Option<Stdio>,
    pub(crate) data: ProcessData,
}

impl Builder {
    pub fn new() -> Self {
        Self {
            stdin: None,
            stdout: None,
            stderr: None,
            data: ProcessData::default()
        }
    }

    /// Set an environment variable in the spawned process. Equivalent to `Command::env`
    pub fn env<K, V>(&mut self, key: K, val: V) -> &mut Self
    where
        K: AsRef<OsStr>,
        V: AsRef<OsStr>,
    {
        self.data.envs
            .insert(key.as_ref().to_owned(), val.as_ref().to_owned());
        self
    }

    /// Set environment variables in the spawned process. Equivalent to `Command::envs`
    pub fn envs<I, K, V>(&mut self, vars: I) -> &mut Self
    where
        I: IntoIterator<Item = (K, V)>,
        K: AsRef<OsStr>,
        V: AsRef<OsStr>,
    {
        self.data.envs.extend(
            vars.into_iter()
                .map(|(k, v)| (k.as_ref().to_owned(), v.as_ref().to_owned())),
        );
        self
    }

    ///
    /// Removes an environment variable in the spawned process. Equivalent to `Command::env_remove`
    pub fn env_remove<K: AsRef<OsStr>>(&mut self, key: K) -> &mut Self {
        self.data.envs.remove(key.as_ref());
        self
    }

    ///
    /// Clears all environment variables in the spawned process. Equivalent to `Command::env_clear`
    pub fn env_clear(&mut self) -> &mut Self {
        self.data.envs.clear();
        self
    }

    ///
    /// Captures the `stdin` of the spawned process, allowing you to manually send data via `JoinHandle::stdin`
    pub fn stdin<T: Into<Stdio>>(&mut self, cfg: T) -> &mut Self {
        self.stdin = Some(cfg.into());
        self
    }

    ///
    /// Captures the `stdout` of the spawned process, allowing you to manually receive data via `JoinHandle::stdout`
    pub fn stdout<T: Into<Stdio>>(&mut self, cfg: T) -> &mut Self {
        self.stdout = Some(cfg.into());
        self
    }

    ///
    /// Captures the `stderr` of the spawned process, allowing you to manually receive data via `JoinHandle::stderr`
    pub fn stderr<T: Into<Stdio>>(&mut self, cfg: T) -> &mut Self {
        self.stderr = Some(cfg.into());
        self
    }

    ///
    /// Before start 
    #[cfg(unix)]
    pub fn before_start<F>(&mut self, f: F) -> &mut Self
    where
        F: FnMut() -> io::Result<()> + Send + Sync + 'static
    {
        self.data.callbacks.before_start = Some(Arc::new(Mutex::new(Box::new(f))));
        self
    }
}
