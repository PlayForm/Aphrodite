//! The registry cell.

use std::sync::{OnceLock, RwLock};

use crate::Template::State::Struct::Struct;

/// The one registry instance for the process, created on first use.
///
/// `RwLock` because reads (every render) vastly outnumber writes (config load,
/// `/reload`, test setup). The lock is poisoned only by a panic inside a
/// writer, and every accessor recovers from that rather than propagating: a
/// poisoned registry must not take down compression.
pub(crate) static Templates:OnceLock<RwLock<Struct>> = OnceLock::new();
