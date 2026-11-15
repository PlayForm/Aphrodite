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

/// flag-file path -> (mtime, enabled) cache; one entry per distinct flag.
pub(crate) static FLAG_CACHE:OnceLock<Mutex<HashMap<String, (f64, bool)>>> = OnceLock::new();
