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
	let (dir, warnings) = Resolve::Fn(home_param);
	for w in &warnings {
		eprintln!("aphrodite-hermes: {w}");
	}
	let mut result = Materialize::Fn(&dir);
	if let Some(arr) = result.get_mut("warnings").and_then(|v| v.as_array_mut()) {
		for w in warnings {
			arr.push(serde_json::Value::String(w));
		}
	}
	result
}
