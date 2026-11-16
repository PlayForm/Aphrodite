//! The process-global registry itself.

use std::{
	collections::HashMap,
	sync::{Arc, Mutex, OnceLock},
};

use aphrodite::State::Struct::AphroditeState;

/// The registry itself. A plain static mutex with poison recovery: every
/// caller holds it for a HashMap lookup/insert and nothing more.
///
/// Nothing may block inside this lock - not a TOML load, not a thread spawn -
/// because it is the one lock every session in the gateway shares. See
/// `Session::Registry::Get::Fn`, which builds a new session's seed OUTSIDE it.
pub(crate) static Store:OnceLock<Mutex<HashMap<String, Arc<Mutex<AphroditeState>>>>> = OnceLock::new();
