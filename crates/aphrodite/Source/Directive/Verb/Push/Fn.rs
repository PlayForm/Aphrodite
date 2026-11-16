//! Append a directive to the active list.

/// Activate `name` unless it is already active.
///
/// Idempotent by design: `add` is called both by a model and by a caller that pre-seeds a
/// set, so a name already in the list must not appear twice - a duplicate would render the
/// directive's body twice in the context string.
pub(crate) fn Fn(Active:&mut Vec<String>, name:&str) {
	let Missing = (!Active.iter().any(|Existing| Existing == name)).then(|| name.to_string());

	Active.extend(Missing);
}
