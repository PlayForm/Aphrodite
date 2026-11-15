//! Unwrapping Hermes's JSON tool-result envelopes.
//!
//! Hermes wraps every tool result in a JSON object (`{"output":...,"exit_code":...}`,
//! `{"success":...,"diff":...}`, `{"total_count":...,"matches":[...]}`). The
//! classifier sees a JSON object and types it as text, hiding the real content
//! behind a useless preview, so the wrapper is unwrapped to pick a better
//! type/preview.
//!
//! This is a HEURISTIC and must never change what a retrieve returns: callers
//! hash and store the ORIGINAL content, and only borrow the classification from
//! here.

mod Fn;
mod Search;

pub(crate) use Fn::Fn;
