//! Tool dispatch - routes Hermes tool calls to aphrodite core functions.
//!
//! Each Hermes tool (`aphrodite_compress`, `aphrodite_retrieve`, ...) has a
//! handler here that parses args, operates on the current session's state
//! (see [`crate::Session::With::Fn::with_shared`]), and returns a JSON result. Tools carry no
//! `session_id` in their args, so [`crate::Session::With::Fn::with_shared`] resolves the
//! session from `Debug::Session::Last::Fn()` (the current turn's session, set by
//! `pre_llm_call`) and locks ONLY that session's state with a bounded lock -
//! content compressed by a hook or `aphrodite_compress` stays resolvable by
//! `aphrodite_retrieve` for that session, and a session whose lock is wedged
//! by an abandoned worker degrades to a throwaway template clone instead of
//! blocking the gateway (fail-open, never block).
//!
//! Layout: `Dispatch/` owns the name -> handler entry point, `Registry/` owns
//! the name -> handler table (one leaf per tool family), `Arg/` the shared
//! argument reader, `Read/` the guarded file read behind
//! `aphrodite_retrieve(path=...)`, `Unwrap/` the Hermes-envelope unwrapper and
//! `Compress/` the store-and-marker step they share. Each leaf owns one item
//! named by its kind; this facade declares children and keeps the two old item
//! names alive for callers.

mod Arg;
mod Compress;
pub(crate) mod Dispatch;
mod Read;
mod Registry;
pub(crate) mod Unwrap;

#[cfg(test)]
mod Test;
