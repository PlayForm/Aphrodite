//! `aphrodite_test` - the in-process compress/retrieve smoke test.
//!
//! The family is named for the tool (`aphrodite_test`); the module's own unit
//! tests live one level up in `Tool/Test/`, so the two never collide.
//!
//! `Const/` is the one sample list, `Mode/` decides how much of it a call runs,
//! and `Fn/` performs the round trip.

pub mod Const;
pub mod Fn;
pub mod Mode;
