//! The debug level - the cheap gate every call site reads.
//!
//! One atomic, three questions: what the level is (`Get/`), set it (`Set/`),
//! and is the logger on at all (`Enabled/`). The storage lives in `Const/` so
//! the three leaves cannot disagree about which atomic they read.

pub mod Const;
pub mod Enabled;
pub mod Get;
pub mod Set;
