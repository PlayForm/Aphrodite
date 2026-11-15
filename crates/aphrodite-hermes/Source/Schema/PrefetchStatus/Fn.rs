//! `aphrodite_prefetch_status` - which prefetched files are resolvable now.
//!
//! Because prefetch is synchronous, this is an inventory rather than a progress
//! bar, and `loading`/`errors` are always empty. Saying that plainly stops a model
//! from polling it waiting for a state change that can never come.

use serde_json::json;

use crate::Schema::NoParams;

/// The `aphrodite_prefetch_status` tool schema.
pub(crate) fn Fn() -> serde_json::Value {
	json!({
		"name": "aphrodite_prefetch_status",
		"description": "List prefetched files that are resolvable now. \
			Because prefetch is synchronous, `loading` and `errors` are always empty and \
			anything tracked is already `ready` - this is an inventory of file-backed \
			markers, not a progress bar. Takes no arguments. \
			Returns {loading: [], ready: [{path, hash, type, size}], errors: [], \
			total_ready}.",
		"parameters": NoParams::Fn()
	})
}
