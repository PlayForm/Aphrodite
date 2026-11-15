//! Join resolution and materialization into the single value the C ABI
//! export returns.
//!
//! `Resolve` produces a directory plus its own warnings; `Materialize`
//! produces the report. This module exists so the two warning streams reach
//! the caller as one array - a resolution warning that never surfaced in the
//! report was invisible to the operator, who only sees the JSON.

use crate::Directive::{Materialize, Resolve};

/// Resolve the target dir and materialize; used by the FFI export.
///
/// Resolution warnings (e.g. a degraded `$HOME` fallback) are logged and
/// folded into the report's `warnings` array.
///
/// It is not a retry loop and holds no state: calling it twice re-runs the
/// whole resolution, which is what makes a second call idempotent rather than
/// additive.
pub(crate) fn Fn(home_param:&str) -> serde_json::Value {
	// `materialize_directives` is the earliest thing the Python loader calls,
	// so it is where a cold process gets its subscriber (`Debug::Init`).
	crate::Debug::Init::Fn::Fn();

	let (dir, warnings) = Resolve::Fn::Fn(home_param);
	warnings.iter().for_each(|w| crate::Debug::Warn::Fn::Fn(w));

	let mut result = Materialize::Fn::Fn(&dir);

	// `Option::iter_mut` is the zero-or-one loop: a report with no
	// `warnings` array simply takes no extra entries.
	result
		.get_mut("warnings")
		.and_then(|v| v.as_array_mut())
		.iter_mut()
		.for_each(|arr| arr.extend(warnings.iter().map(|w| serde_json::Value::String(w.clone()))));

	result
}
