//! The complete, ordered tool catalogue the model is shown.
//!
//! Order is meaningful: the deferred-catalog listing and `tool_describe` both
//! read this array, so the position of a tool is its position in the model's
//! tool list. The dev-only `aphrodite_debug` schema is appended last and only
//! under `debug_assertions`, because a release dylib that advertises it fails
//! catalog admission as an undeclared tool.

use crate::Schema::{
	Catalog,
	Compress,
	Diff,
	Directive,
	File,
	Prefetch,
	PrefetchStatus,
	Rebuild,
	Reclassify,
	Retrieve,
	Search,
	Smoke,
	Stats,
};
#[cfg(debug_assertions)]
use crate::Schema::Debug;

/// Return all tool schemas as a JSON array.
///
/// It allocates a fresh array on every call - the schemas are cheap `json!`
/// literals, and a cache would have to be invalidated by a build profile, which
/// is exactly the kind of state that goes stale.
pub fn Fn() -> Vec<serde_json::Value> {
	vec![
		Compress::Fn(),
		Retrieve::Fn(),
		Stats::Fn(),
		File::Fn(),
		Diff::Fn(),
		Search::Fn(),
		Directive::Fn(),
		Smoke::Fn(),
		Catalog::Fn(),
		Reclassify::Fn(),
		Prefetch::Fn(),
		PrefetchStatus::Fn(),
		Rebuild::Fn(),
		#[cfg(debug_assertions)]
		Debug::Fn(),
	]
}
