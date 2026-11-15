//! The request context: what the pipeline knows about one request, bundled.
//!
//! Ten positional arguments crossed four functions - the state, the method, the
//! path, the request id, the start instant, the inbound headers, the cache key,
//! the route flag, the upstream result and the timeout flag - so a reader could
//! not tell which of them a stage actually used, and the responder carried
//! `#[allow(clippy::too_many_arguments)]` to make the arity legal. The context
//! is the request half of that set; the upstream half travels separately
//! because it is produced after the request is understood.

use std::{sync::Arc, time::Instant};

use axum::http::{HeaderMap, Method};

use crate::Proxy::{Request::Semantics::Struct::Struct as RequestSemantics, State::Struct::Struct as AppState};

/// One request, as the stages below `Route` read it.
pub(crate) struct Struct<'a> {
	/// Shared proxy state.
	pub(crate) State:&'a Arc<AppState>,
	/// The inbound method, forwarded to the upstream and recorded.
	pub(crate) Method:&'a Method,
	/// The inbound path without the query string.
	pub(crate) Path:&'a str,
	/// The eight-character request id used by every log line.
	pub(crate) ID:&'a str,
	/// When the request was accepted; every latency figure is measured from here.
	pub(crate) Started:Instant,
	/// The inbound headers, forwarded to the upstream and read for the budget.
	pub(crate) Headers:&'a HeaderMap,
	/// The single interpretation of the body, produced before this value.
	pub(crate) Semantics:RequestSemantics,
}
