//! Shared fixtures for the proxy tests.
//!
//! One file per fixture family; the facade re-exports each fixture under the
//! name the tests call it by.

pub mod Chat;
pub mod Lock;
pub mod State;
pub mod Store;
pub mod Upstream;
