//! The `/tool_relay` handler's body: validate, dispatch, answer.
//!
//! Split from `Fn` only so the relay span can be entered with `Instrument`.
//! A request carrying a `CallbackURL` is spawned onto the task tracker and
//! answered immediately, so the caller is never held open by a slow tool; a
//! request without one is executed inline.
//!
//! The two rejections are `let ... else` rather than nested matches: each is a
//! guard on a value that is either acceptable or not, and the else branch is the
//! whole rejection. The callback's scheme check is a `filter` on the parsed URL,
//! because "parse and require https" is one predicate, not two decisions.

use std::{
	sync::{Arc, atomic::Ordering},
	time::Duration,
};

use axum::{
	http::StatusCode,
	response::{IntoResponse, Json, Response},
};

use crate::Proxy::{
	Relay::{
		Execute::Fn::Fn as Execute,
		Handle::Route::Sync::Fn::Fn as ExecuteSynchronously,
		Request::Struct::Struct as RelayRequest,
		Response::Struct::Struct as RelayResponse,
	},
	State::Struct::Struct as AppState,
};

pub(crate) async fn Fn(state:Arc<AppState>, Request:RelayRequest) -> Response {
	state.RelayCalls.fetch_add(1, Ordering::Relaxed);
	tracing::info!(Tool = %Request.Tool, "tool_relay");

	// Validate aphrodite_retrieve: a request with only `Query` and no `hash` is
	// invalid and must return 400 instead of silently passing through.
	let Acceptable =
		Request.Params.get("hash").and_then(|Value| Value.as_str()).is_some() || Request.Tool != "aphrodite_retrieve";
	let Some(()) = Acceptable.then_some(()) else {
		return (
			StatusCode::BAD_REQUEST,
			Json(RelayResponse {
				Success:false,
				Result:None,
				error:Some(
					"`hash` is required for 💋/aphrodite_retrieve. Requests with only `query` and no `hash` are \
					 invalid."
						.into(),
				),
				AsyncCall:false,
			}),
		)
			.into_response();
	};

	let Some(CallbackURL) = Request.CallbackURL.as_deref() else {
		return ExecuteSynchronously(&state, &Request.Tool, &Request.Params).await;
	};

	// SSRF protection: only https:// URLs allowed.
	let Some(ParsedURL) = url::Url::parse(CallbackURL).ok().filter(|URL| URL.scheme() == "https") else {
		// F13: report the rejection honestly instead of `success:true` with
		// nothing executed - a caller passing e.g. a loopback `http://` callback
		// previously got told it worked and then waited forever for a callback
		// that would never arrive.
		tracing::warn!(CallbackURL = %CallbackURL, "tool_relay callback rejected: only https scheme allowed");

		return (
			StatusCode::BAD_REQUEST,
			Json(RelayResponse {
				Success:false,
				Result:None,
				error:Some("callback_url must use the https scheme".into()),
				AsyncCall:false,
			}),
		)
			.into_response();
	};

	let Tracker = state.TaskTracker.clone();
	let state = state.clone();
	let Tool = Request.Tool.clone();
	let Params = Request.Params.clone();
	let Cb = ParsedURL.to_string();

	Tracker.spawn(async move {
		// The callback body is the same `Result<Value, String>` shape the
		// synchronous path returns, so an `{"Ok": ...}` / `{"Err": "..."}`
		// envelope reaches the callback URL either way; only the error's
		// representation changes, and `{e:#}` keeps the whole anyhow chain.
		let Result = Execute(&state, &Tool, &Params).await.map_err(|e| format!("{e:#}"));
		// F13: the execute result itself was never counted on this async path -
		// only the synchronous path incremented these, so
		// `tool_relay_success + tool_relay_failure` silently diverged from
		// `RelayCalls` for every async callback request.
		let _ = Result
			.is_ok()
			.then_some(&state.RelaySuccess)
			.unwrap_or(&state.RelayFailure)
			.fetch_add(1, Ordering::Relaxed);

		let _ = state
			.Client
			.post(&Cb)
			.json(&Result)
			.timeout(Duration::from_secs(5))
			.send()
			.await;
	});

	Json(RelayResponse { Success:true, Result:None, error:None, AsyncCall:true }).into_response()
}
