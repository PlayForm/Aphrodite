//! The process-global debug level mirror.

use std::sync::atomic::AtomicU8;

/// Process-global debug level mirror (0/1/2), set from
/// `AphroditeState::debug_level` at config load so stateless render points
/// (templates, marker format) gate on the same flag.
///
/// A relaxed atomic on purpose: the flag is written once at session start and
/// read on every transform, and nothing downstream needs an ordering
/// guarantee stronger than "visible within the process". A stronger ordering
/// would cost every transform to protect a value a turn boundary already
/// publishes.
pub(crate) static Level:AtomicU8 = AtomicU8::new(0);
