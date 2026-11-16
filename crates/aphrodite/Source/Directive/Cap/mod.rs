//! The directive size caps.
//!
//! `Fn/` cuts one directive body to the per-file cap and the two leaves hold the numbers:
//! `Single/` is the cap on one directive's body and `Combined/` is the cap on the text
//! they stack up to. Both are characters, and each is applied by one function, so a cap
//! can only be wrong in one place.

pub mod Combined;
pub mod Fn;
pub mod Single;
