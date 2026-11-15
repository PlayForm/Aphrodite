//! The whole `aphrodite.toml` document, and what is done with it.
//!
//! `Struct` is the parsed document; `Load` reads one from a path and `Resolve`
//! turns one `[[proxies]]` entry into the `CLI` the proxy runs on. `Port` is
//! the per-mode override `Resolve` applies last.

pub mod Load;
pub mod Port;
pub mod Resolve;
pub mod Struct;
