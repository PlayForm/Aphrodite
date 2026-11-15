//! Marker rendering.
//!
//! A marker is built in one direction only: the entry point sanitizes the
//! preview, the block renderer picks a template or the fallback, and the
//! fallback is the one place that knows the canonical grammar. Splitting these
//! apart is what makes "which grammar did this marker use?" answerable by
//! looking at one file.
//!
//! - `Const.rs` - the process-global template registration and its hint knob.
//! - `Entry.rs` - `CCRMarker`, the public entry point.
//! - `Block.rs` - configured-template vs fallback dispatch.
//! - `Fallback.rs` - the canonical `<<<CCR:...>>>` grammar.
//! - `Bracket.rs` - "is this preview already self-describing?".
//! - `Vars.rs` - `{name}` substitution for TOML templates.

pub mod Bracket;
pub mod Block;
pub mod Const;
pub mod Entry;
pub mod Fallback;
pub mod Vars;
