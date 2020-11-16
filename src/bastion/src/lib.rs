//! # Bastion: Fault-tolerant Runtime for Rust applications
//!
// FIXME: Update the documentation

#![doc(
    html_logo_url = "https://raw.githubusercontent.com/bastion-rs/bastion/master/img/bastion-logo.png"
)]
// Force missing implementations
#![warn(missing_docs)]
#![warn(missing_debug_implementations)]
// Deny using unsafe code
#![deny(unsafe_code)]
// Doc generation experimental features
#![cfg_attr(feature = "docs", feature(doc_cfg))]

mod actor;
mod routing;
