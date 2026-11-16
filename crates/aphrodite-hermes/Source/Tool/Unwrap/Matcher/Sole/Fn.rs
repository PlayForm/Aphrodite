//! The sole-key narrowing itself.

use serde_json::{Map, Value};

/// The object, when it carries at most one field.
///
/// It returns the borrow it was given - this narrows, it does not copy or
/// normalise, so a matcher pays nothing for the guard that rejects most
/// payloads. An empty object is admitted here and rejected by the caller's own
/// `get`: this leaf answers "is it a sole-field object?", never "does that field
/// hold a payload?".
pub(crate) fn Fn(Object:&Map<String, Value>) -> Option<&Map<String, Value>> { (Object.len() <= 1).then_some(Object) }
