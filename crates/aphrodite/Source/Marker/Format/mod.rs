//! Marker rendering.
//!
//! A marker is built in one direction only: the entry point sanitizes the
//! preview, the block renderer picks a template or the fallback, and the
//! fallback is the one place that knows the canonical grammar. Splitting these
//! apart is what makes "which grammar did this marker use?" answerable by
//! looking at one file.
//!
//! - `Registration/` - the process-global registration and its hint, published
//!   as one value.
//! - `Grammar/` - which of the two grammars is in force.
//! - `Budget/` - the headroom budget's percentage as a truncation.
//! - `Entry.rs` - `Fn`, the public entry point.
//! - `Block.rs` - `Fn`, configured-template vs fallback dispatch.
//! - `Fallback.rs` - `Fn`, the canonical `<<<CCR:...>>>` grammar.
//! - `Bracket.rs` - `Fn`, "is this preview already self-describing?".
//! - `Vars.rs` - `Fn`, `{name}` substitution for TOML templates.
//!
//! Every leaf exports one `Fn`, because the directory already names what it acts
//! on; the kind is the name, the path is the namespace.

pub mod Block;
pub mod Bracket;
pub mod Budget;
pub mod Entry;
pub mod Fallback;
pub mod Grammar;
pub mod Registration;
pub mod Vars;
