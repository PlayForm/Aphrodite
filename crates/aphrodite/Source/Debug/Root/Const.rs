//! The root session id storage.

use std::sync::{Mutex, OnceLock};

/// The most recently seen ROOT session id (published by the Hermes bridge's
/// `record_session`). Persisted across process restarts via the
/// `session.current` file, which the bridge also writes.
///
/// A `Mutex<String>` rather than an atomic pointer: the id is written once per
/// session and read on every log line, and a `String` behind a mutex keeps the
/// read path allocation-free without leaking a boxed value per session.
pub(crate) static RootSession:OnceLock<Mutex<String>> = OnceLock::new();
