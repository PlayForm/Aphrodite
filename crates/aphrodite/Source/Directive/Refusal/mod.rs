//! The directive vocabulary's refusals, in one place.
//!
//! `Enum/` names the two questions a refusal answers and states the whole tolerance policy for
//! the directive surface (including the case that is deliberately NOT refused); `Fn/` renders
//! either of them as the single error shape the ABI returns.

pub mod Enum;
pub mod Fn;
