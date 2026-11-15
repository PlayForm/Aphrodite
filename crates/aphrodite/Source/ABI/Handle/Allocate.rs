//! Handle allocation: minting the opaque session handle `AphroditeInit`
//! returns and `AphroditeDestroy` releases.
//!
//! The ID is a counter, not a pointer and not a hash: it must be
//! representable as a decimal C string, and reusing a freed ID must never
//! resurrect the destroyed session.

use std::sync::{Arc, Mutex};

use super::Table::Handles;
use crate::State::Struct::AphroditeState;

/// Next handle ID to hand out from `AllocHandle`. Wraps on overflow rather
/// than panicking - see the `wrapping_add` call there.
pub(crate) static NextID:Mutex<usize> = Mutex::new(1);

/// Allocate a fresh handle for `State` and insert it into the table.
///
/// The returned ID is what the caller receives as a decimal string. ID reuse is
/// only possible after `usize` wraps, which is why the counter wraps instead of
/// saturating: a saturated counter would hand out one ID forever and silently
/// alias every later session onto the first one's state.
pub(crate) fn AllocHandle(State:AphroditeState) -> usize {
	let mut ID = NextID.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
	let Hid = *ID;
	*ID = ID.wrapping_add(1); // overflow-safe
	Handles().as_mut().unwrap().insert(Hid, Arc::new(Mutex::new(State)));
	Hid
}
