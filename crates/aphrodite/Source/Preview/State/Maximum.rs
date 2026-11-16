//! The cap value itself.

use std::sync::atomic::AtomicU32;

/// The cap, in characters; `0` = unlimited.
///
/// The external contract is `[previews] preview_max_chars` (environment
/// `APHRODITE_PREVIEW_MAX_CHARS` overrides the TOML key; an absent key means unlimited).
/// The key was declared in the config structs long before anything read it, so every
/// preview knob was a no-op; the builder now enforces this value on EVERY path - proxy,
/// hooks and the Hermes dylib all funnel through the preview entry point.
///
/// A `static` rather than a `const`: the value changes at runtime on config reload,
/// and it is read on every preview. It is not per-session and must not become so -
/// one process, one cap.
pub(crate) static Value:AtomicU32 = AtomicU32::new(0);
