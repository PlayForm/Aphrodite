//! Lookup of one schema by tool name.
//!
//! `tool_describe` and `aphrodite_hermes_get_schema` both need exactly this, and
//! both need it to agree with the catalogue the model was shown - so it searches
//! `All::Fn::Fn` rather than keeping an index that could drift.

use crate::Schema::All;

/// Get a single tool schema by name.
///
/// Returns `None` for an unknown name, which the FFI layer turns into
/// `{"error": "unknown tool: <name>"}`. It is a linear scan on purpose: the
/// catalogue is small and a duplicate-name bug would be masked by a map.
pub fn Fn(Name:&str) -> Option<serde_json::Value> { All::Fn::Fn().into_iter().find(|Schema| Schema["name"] == Name) }
