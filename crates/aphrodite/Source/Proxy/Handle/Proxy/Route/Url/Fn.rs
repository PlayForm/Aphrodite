//! Build the upstream URL for a request.
//!
//! F10: the query string is forwarded too - `path.path()` excludes it, so any
//! OpenAI-compatible endpoint using query params (e.g. `GET /v1/models?limit=5`)
//! had it silently dropped by the catch-all.

use std::sync::Arc;

use crate::Proxy::State::Struct::Struct as AppState;

/// The upstream URL: the configured API base plus the inbound path and query.
pub(crate) fn Fn(State:&Arc<AppState>, Path:&axum::extract::OriginalUri) -> String {
	let Suffix = Path
		.0
		.path_and_query()
		.map(|Pair| Pair.as_str())
		.unwrap_or_else(|| Path.path())
		.trim_start_matches('/');

	format!("{}/{}", State.APIURL.trim_end_matches('/'), Suffix)
}
