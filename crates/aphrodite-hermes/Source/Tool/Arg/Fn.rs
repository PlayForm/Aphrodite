//! The shared string-argument reader.
//!
//! Returns a borrow, not an owned `String`: handlers pass these straight into
//! core functions, and the args object outlives every use.

use serde_json::Value;

/// Read `Key` from `Args` as a string, or `""` when absent or not a string.
///
/// It is not a validator: a missing required argument becomes the empty string,
/// which each handler then rejects with its own message ("content is required",
/// "hash or path is required"). That split is deliberate - only the handler knows
/// which fields are required and how to say so.
pub fn Fn<'a>(Args:&'a Value, Key:&str) -> &'a str { Args.get(Key).and_then(|Value| Value.as_str()).unwrap_or("") }
