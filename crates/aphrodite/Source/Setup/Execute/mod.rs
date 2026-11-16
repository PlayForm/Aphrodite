//! Executing an installation plan.
//!
//! `Fn` is the interpreter: it matches a `Step` and calls the one leaf that performs
//! it. Every step is its own directory, so a step can be read, changed and reasoned
//! about without the others - and the order lives in the plan, not in the order of
//! statements in a function.

pub mod Binary;
pub mod Configuration;
pub mod Directory;
pub mod Failure;
pub mod Fn;
pub mod Plugin;
pub mod Registration;
pub mod Report;
