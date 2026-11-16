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
/// folded into the report's `warnings` array - the report is the value the
/// operator actually sees, and a resolution warning that never surfaced in it was
/// invisible to them.
///
/// It is not a retry loop and holds no state: calling it twice re-runs the
/// whole resolution, which is what makes a second call idempotent rather than
/// additive.
pub(crate) fn Fn(Override:&str) -> serde_json::Value {
	// `materialize_directives` is the earliest thing the Python loader calls,
	// so it is where a cold process gets its subscriber (`Debug::Init`).
	crate::Debug::Init::Fn::Fn();

	let (Dir, Warnings) = Resolve::Fn::Fn(Override);
	Warnings.iter().for_each(|Warning| crate::Debug::Warn::Fn::Fn(Warning));

	let mut Report = Materialize::Fn::Fn(&Dir);

	// The report is a value, so the warnings join it by `extend` - the previous
	// `get_mut("warnings")`/`Option::iter_mut` walk was a zero-or-one loop over a
	// key that is always there.
	Report.Warnings.extend(Warnings);

	Report.Json()
}
