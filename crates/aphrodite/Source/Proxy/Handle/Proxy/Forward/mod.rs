//! `Forward` - send the upstream request, with the connect-only retry policy.
//!
//! The decision is pure (`Decide`), the policy is data (`Policy::Const`), the
//! request construction is its own leaf (`Send`), the failure is a typed value
//! (`Failure`), and `Fn` is the interpreter that loops, sleeps and returns.
//! Splitting them is what lets the safety policy be read without reading the
//! retry loop.

pub mod Decide;
pub mod Decision;
pub mod Failure;
pub mod Fn;
pub mod Policy;
pub mod Send;
