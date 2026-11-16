//! The caller-string reader, its decoded shape, and the two ways an export
//! resolves it.
//!
//! One pointer in, one `Enum` out, and then exactly one decision per export: the
//! argument was optional (`Optional`) or required (`Required`). Keeping that
//! decision out of the decoder is what makes "the caller passed nothing" and "the
//! caller passed something no export can serve" two different values instead of
//! one `None`.

pub mod Enum;
pub mod Fn;
pub mod Optional;
pub mod Required;
