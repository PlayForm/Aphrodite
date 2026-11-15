//! The tool-relay surface: the wire types and the handler that consumes
//! them.
//!
//! Relay is the only bidirectional path in the proxy - Hermes asks the proxy to
//! run a tool against its own CCR state. Request, response and handler are
//! separate leaves because the two structs are a wire contract and the handler
//! is free to change.

pub mod Execute;
pub mod Handle;
pub mod Request;
pub mod Response;
