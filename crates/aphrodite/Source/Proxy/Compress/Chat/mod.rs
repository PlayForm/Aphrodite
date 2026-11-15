//! Rewriting the tool output inside a Chat Completions response.
//!
//! `Plan` decides, `Apply` writes, `Fn` is the two together. Nothing here
//! injects the `AphroditeRetrieve` tool definition into `tool_calls` - that was
//! tried and reverted (Bug 18) because it makes the upstream see a tool it never
//! advertised; the Python plugin registers the tool instead.

pub mod Apply;
pub mod Fn;
pub mod Outcome;
pub mod Plan;
pub mod Rewrite;
pub mod Sizing;
