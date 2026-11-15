//! Read the preview length cap.

use std::sync::atomic::Ordering;

use super::Maximum::PreviewMaximumChars;

/// The current preview cap in characters; 0 means unlimited.
///
/// A visibility helper for tests and for the configuration round trip, not a
/// decision point - the builder reads the atomic directly so it cannot observe a
/// different value than the one it enforces.
pub fn Fn() -> u32 { PreviewMaximumChars.load(Ordering::Relaxed) }
