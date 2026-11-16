//! Read the preview length cap.

use std::sync::atomic::Ordering;

use super::Maximum::Value;

/// The current cap, in characters; `0` = unlimited (`[previews] preview_max_chars`).
///
/// A visibility helper for tests and for the configuration round trip, not a
/// decision point - the builder reads the atomic directly so it cannot observe a
/// different value than the one it enforces.
pub fn Fn() -> u32 { Value.load(Ordering::Relaxed) }
