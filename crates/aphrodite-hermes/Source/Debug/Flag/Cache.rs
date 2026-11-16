//! The mtime cache in front of the flag files.
//!
//! `enabled_for` runs on every compressed tool result, so a stat() and a read
//! per call would be pure overhead on the quiet path. The cache is keyed by
//! path and validated by mtime: an unchanged file costs one stat, a touched file
//! costs one read.

use std::{
	collections::HashMap,
	sync::{Mutex, OnceLock},
};

use crate::Debug::Flag::State;

/// flag-file path -> (mtime, state) cache; one entry per distinct flag.
///
/// The state is three-valued (on / off / absent) because the reader's answer is:
/// caching a `bool` would make the cache unable to say "this file does not
/// exist", which is the answer the resolution walks for.
pub(crate) static FLAG_CACHE:OnceLock<Mutex<HashMap<String, (f64, State::Enum::Enum)>>> = OnceLock::new();
