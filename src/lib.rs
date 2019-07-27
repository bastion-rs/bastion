// Because if we can't trust, we can't make.
#![forbid(unsafe_code)]
#![feature(const_fn)]
#![feature(unboxed_closures)]
#![feature(fn_traits)]
#![feature(clamp)]
#![feature(panic_info_message)]

#[macro_use]
extern crate log;
extern crate env_logger;

pub mod receive;
pub mod child;
pub mod context;

pub mod messages;
mod runtime_manager;
pub mod spawn;
pub mod supervisor;
pub mod tramp;

pub mod bastion;
pub mod config;
pub mod runtime_system;

pub mod macros;
