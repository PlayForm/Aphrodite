//! `Level`: the current debug level.

use std::sync::atomic::Ordering;

use crate::Debug::Level::Const::Level;

/// The current debug level - the cheap gate (one relaxed atomic load).
///
/// This is the function every hot path calls, so it must stay a single load: a
/// call site that instead re-read the environment would pay a syscall per
/// transform and could disagree with the level set at session start.
#[inline]
pub fn Fn() -> u8 { Level.load(Ordering::Relaxed) }
