//! The one-shot first-turn orientation.
//!
//! Rendered exactly once, on turn 0, then never again: it is the only place the
//! model is told the session runs under CCR at all. Everything it says about
//! markers is vocabulary for using them, not an explanation of how they are
//! produced.

pub mod Fn;
