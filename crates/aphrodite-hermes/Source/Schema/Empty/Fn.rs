//! The parameter block shared by every argument-less tool.
//!
//! One definition rather than thirteen copies: the `additionalProperties: false`
//! flag is the only signal that stops a model from inventing plausible-looking
//! arguments for an argument-less tool, and a copy that lost it would fail
//! silently (the backend either honours the flag or strips it harmlessly, so
//! nothing errors either way).

use serde_json::json;

/// The `parameters` object every argument-less tool schema emits.
///
/// `additionalProperties: false` is deliberate: it is the only signal that
/// stops a model from inventing plausible-looking arguments for an argument-
/// less tool, and every backend either honors it or strips it harmlessly.
///
/// It is not `{}`: an empty object without the flag accepts any argument a model
/// dreams up, which is the failure this leaf exists to prevent.
pub(crate) fn Fn() -> serde_json::Value { json!({"type": "object", "properties": {}, "additionalProperties": false}) }
