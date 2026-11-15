//! The catch-all proxy handler and its stages.
//!
//! `Fn` opens the span and `Route` is its body; the request half of what those
//! stages read lives in `Request::Context`, and the forward and respond stages
//! each own one failure mode.

pub mod Fn;
pub mod Forward;
pub mod Respond;
pub mod Route;
