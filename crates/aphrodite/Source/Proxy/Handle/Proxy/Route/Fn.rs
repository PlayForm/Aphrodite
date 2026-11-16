//! The catch-all proxy handler's body: one request, start to finish.
//!
//! Split from `Fn` only so the request span can be entered with `Instrument`
//! (an async fn cannot hold an `Entered` guard across its awaits). The sequence
//! is: count the request, interpret the body once, log it in dev mode, answer it
//! from the cache when it may be, otherwise forward and respond.
//!
//! The interpretation happens before anything branches, and the cache decision
//! and the upstream client both read it - so a request can no longer be
//! streaming to one and cacheable to the other (D3).
//!
//! The cache lookup and the forward are the two arms of one `match`, not a
//! lookup followed by an early return: a hit IS the response and a bypass or a
//! miss both take the upstream road, so there is no third state to fall through.

use std::sync::{Arc, atomic::Ordering};

use axum::http::Method;
use bytes::Bytes;

use crate::Proxy::{
	Body::Truncate::Fn::Fn as TruncateCharSafe,
	Cache::Lookup::{Enum::Enum as CacheLookup, Fn::Fn as LookupCache},
	Handle::Proxy::Route::Send::Fn::Fn as SendUpstream,
	Log::Dev::Fn::Fn as DevLog,
	Request::{Context::Struct::Struct as RequestContext, Semantics::Fn::Fn as InterpretRequest},
	State::Struct::Struct as AppState,
};

pub(crate) async fn Fn(
	state:Arc<AppState>,
	Method:Method,
	path:axum::extract::OriginalUri,
	Headers:axum::http::HeaderMap,
	body:Bytes,
) -> axum::response::Response {
	state.RequestsTotal.fetch_add(1, Ordering::Relaxed);
	state.RequestBodyBytes.fetch_add(body.len() as u64, Ordering::Relaxed);
	let Started = std::time::Instant::now();
	let RequestID = uuid::Uuid::new_v4().to_string();
	let ID = RequestID.get(..8).unwrap_or(&RequestID);

	// One interpretation of the body, produced before anything branches on it.
	let Semantics = InterpretRequest(path.path(), &body, state.APIKey.Expose());

	DevLog(state.Dev, || {
		let mut HdrLog = String::new();

		Headers.iter().for_each(|(Name, Value)| {
			let Rendered = if Name.as_str() != "authorization" {
				format!("  {}: {}", Name.as_str(), TruncateCharSafe(Value.to_str().unwrap_or("?"), 80))
			} else {
				"  authorization: ***".to_string()
			};

			HdrLog.push_str(&Rendered);
			HdrLog.push('\n');
		});

		tracing::info!(
			ID = %ID,
			Method = %Method,
			path = %path.path(),
			BodyLength = body.len(),
			BodyKind = Semantics.Body.Kind(),
			Headers = %HdrLog,
			">>> REQ"
		);
	});

	let Context = RequestContext {
		State:&state,
		Method:&Method,
		Path:path.path(),
		ID,
		Started,
		Headers:&Headers,
		Semantics,
	};

	match LookupCache(&Context, std::time::Instant::now()) {
		CacheLookup::Hit(Cached) => Cached,
		// A miss names the key it looked under; a bypass has no key to name.
		// Both take the upstream road - the difference is already recorded by the
		// lookup itself.
		CacheLookup::Miss { Identity } => {
			// The digest is the identity's index - a short, stable handle for one
			// request, which is exactly what a dev log wants.
			DevLog(
				state.Dev,
				|| tracing::info!(ID = %ID, CacheKey = Identity.Key(), "<<< CACHE MISS"),
			);

			SendUpstream(&Context, &path, body).await
		},
		CacheLookup::Bypassed => SendUpstream(&Context, &path, body).await,
	}
}
