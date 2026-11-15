//! The process-global handle table every stateful export resolves through.
//!
//! `HandleTable` is the single piece of shared mutable state in the ABI: it
//! maps a caller's handle to that session's state. It is deliberately the only
//! global mutex - per-session work happens under the per-session lock, never
//! here.

use std::{
	collections::HashMap,
	sync::{Arc, Mutex},
};

use crate::State::Struct::AphroditeState;

/// Process-global handle table. `None` until the first handle is allocated,
/// so `Mutex::new` can stay `const` without requiring a heap alloc at
/// startup; `Handles()` lazily initializes it to `Some` on first use.
///
/// Each session's state lives behind its own `Arc<Mutex<..>>` (F4/T6, report
/// 06) rather than directly in the map: the outer `HandleTable` mutex is only
/// ever held long enough to look up and clone that `Arc`, never across a
/// hook/tool body. Previously one global lock serialized every session's
/// every call - two different agent sessions could not classify/compress
/// concurrently at all, even though they share nothing.
pub(crate) static HandleTable:Mutex<Option<HashMap<usize, Arc<Mutex<AphroditeState>>>>> = Mutex::new(None);

/// Hand the table out behind its lock, initialising it on first use.
///
/// The returned guard is the global lock: hold it for a map operation and
/// nothing else. A poisoned lock is recovered rather than propagated - one
/// panicking hook must not make every later call on every other handle fail.
pub(crate) fn Handles() -> std::sync::MutexGuard<'static, Option<HashMap<usize, Arc<Mutex<AphroditeState>>>>> {
	let mut g = HandleTable.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
	if g.is_none() {
		*g = Some(HashMap::new());
	}
	g
}
