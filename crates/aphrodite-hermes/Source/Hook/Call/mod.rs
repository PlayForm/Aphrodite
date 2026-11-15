//! One hook call: decode, dispatch under the session's bounded lock, encode.
//!
//! `decode` -> `dispatch` -> `encode` is the whole pipeline, and `Error/`
//! supplies the value returned when `dispatch` reports a wedged session lock.
//! The stage split exists so the arm bodies stay readable; it must not change
//! the ORDER of operations - the session id is resolved before any lock is
//! taken, and the fail-open value is computed only after `dispatch` has
//! already returned.

pub mod Decode;
pub mod Dispatch;
pub mod Encode;
pub mod Error;
pub mod Fn;
pub mod Input;
