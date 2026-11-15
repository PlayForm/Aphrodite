//! The catch-all proxy handler - the hot path of the whole binary.
//!
//! Entry point only: it opens the request span and hands the request to
//! `Route`, which counts it, logs it in dev mode, builds the upstream URL and
//! the cache key, then delegates to the three stages - `Lookup` (answer from
//! cache), `Forward` (upstream with retry) and `Respond` (stream, compress or
//! fail). The stages are separate files because each one is a distinct failure
//! mode, and the handler itself stays readable.
//!
//! The span is entered through `Instrument`, not an `Entered` guard: this is an
//! async fn, and a guard held across an await point would attribute whichever
//! task the executor runs next to this request.

use std::sync::Arc;

use axum::{extract::State, http::Method, response::IntoResponse};
use tracing::Instrument;

use super::Route::Fn::Fn as Route;
use crate::Proxy::{Body::Capped::Struct::Struct as CappedBody, State::Struct::Struct as AppState};

pub async fn Fn(
	State(state):State<Arc<AppState>>,
	Method:Method,
	path:axum::extract::OriginalUri,
	Headers:axum::http::HeaderMap,
	CappedBody(body):CappedBody,
) -> impl IntoResponse {
	// `Method` and `path` identify the work; `body_bytes` says how much was
	// sent without putting a single byte of the payload - a prompt or an API
	// key - into the trace.
	let Span = tracing::info_span!(
		"proxy.request",
		Method = %Method,
		path = %path.path(),
		body_bytes = body.len(),
	);
	Route(state, Method, path, Headers, body).instrument(Span).await
}
