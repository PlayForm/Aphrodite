//! Which CCR type a terminal output deserves.
//!
//! Terminal outputs are often short and do not trigger the headroom
//! classifier's build-output pattern, so the marker lists below give cargo
//! output, test runs and shell traces meaningful previews instead of a generic
//! `text` label.

pub mod Fn;
pub mod Kind;
pub mod Resolve;
