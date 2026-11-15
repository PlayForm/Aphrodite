//! Test battery for the structure extractor, split by the area it pins.
//!
//! The split follows what each file proves: `Detect` that a language is recognised
//! from content alone, one file per language that its declarations are read and
//! truncated the way the marker assumes, `Format` that a multi-byte cut cannot panic
//! the preview path, and `Budget` that a large file is capped rather than flooding the
//! marker. A failure names the invariant that broke without needing the line number.

mod Budget;
mod Detect;
mod Format;
mod Go;
mod JS;
mod Python;
mod Rust;
