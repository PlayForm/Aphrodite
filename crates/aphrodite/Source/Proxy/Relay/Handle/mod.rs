//! `Handle` - The `/tool_relay` handler: validate, dispatch, answer.
//!
//! `Fn` opens the relay span and `Route` is its body; this facade only declares
//! them.

pub mod Fn;
pub mod Route;
