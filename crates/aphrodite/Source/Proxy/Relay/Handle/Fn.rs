//! The `/tool_relay` handler: validate, dispatch, answer.
//!
//! Synchronous calls run inline and return their result; a request carrying a
//! `CallbackURL` is spawned onto the task tracker and answered immediately, so
//! the caller is never held open by a slow tool. Both paths count into the
//! relay counters.

use std::{
	sync::{Arc, atomic::Ordering},
	time::Duration,
};

use axum::{
	extract::State,
	http::StatusCode,
	response::{IntoResponse, Json},
};

use crate::Proxy::{
	Relay::{
		Execute::Fn::Fn as ExecuteToolRelay,
		Request::Struct::Struct as ToolRelayRequest,
		Response::Struct::Struct as ToolRelayResponse,
	},
	State::Struct::Struct as AppState,
};

/// `POST /tool_relay` - dispatches a Hermes-side tool call (see
/// [`ExecuteToolRelay`]). Runs synchronously unless the request carries
/// an `https://` `CallbackURL`, in which case it's spawned onto
/// `TaskTracker` and the result is POSTed back later instead of returned
/// inline.
pub async fn Fn(State(state):State<Arc<AppState>>, Json(Request):Json<ToolRelayRequest>) -> impl IntoResponse {
	state.ToolRelayCalls.fetch_add(1, Ordering::Relaxed);
	tracing::info!(Tool = %Request.Tool, "tool_relay");

	// Validate aphrodite_retrieve: requests with only `Query` and no `hash` are
	// invalid and must return 400 BAD_REQUEST instead of silently passing through.
	if Request.Tool == "aphrodite_retrieve" && Request.Params.get("hash").and_then(|v| v.as_str()).is_none() {
		return (
			StatusCode::BAD_REQUEST,
			Json(ToolRelayResponse {
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
	}

	if let Some(Cb) = &Request.CallbackURL {
		// SSRF protection: only https:// URLs allowed
		let ParsedURL = match url::Url::parse(Cb) {
			Ok(u) if u.scheme() == "https" => u,
			_ => {
				// F13: report the rejection honestly instead of `success:true`
				// with nothing executed - a caller passing e.g. a loopback
				// `http://` callback previously got told it worked and then
				// waited forever for a callback that would never arrive.
				tracing::warn!(CallbackURL = %Cb, "tool_relay callback rejected: only https scheme allowed");
				return (
					StatusCode::BAD_REQUEST,
					Json(ToolRelayResponse {
						Success:false,
						Result:None,
						error:Some("callback_url must use the https scheme".into()),
						AsyncCall:false,
					}),
				)
					.into_response();
			},
		};
		let Tracker = state.TaskTracker.clone();
		let state = state.clone();
		let Tool = Request.Tool.clone();
		let Params = Request.Params.clone();
		let Cb = ParsedURL.to_string();
		Tracker.spawn(async move {
			let Result = ExecuteToolRelay(&state, &Tool, &Params).await;
			// F13: the execute result itself (success/failure of the tool
			// call) was never counted on this async path - only the
			// synchronous path below incremented these, so
			// `tool_relay_success + tool_relay_failure` silently diverged
			// from `ToolRelayCalls` for every async callback request.
			if Result.is_ok() {
				state.ToolRelaySuccess.fetch_add(1, Ordering::Relaxed);
			} else {
				state.ToolRelayFailure.fetch_add(1, Ordering::Relaxed);
			}
			let _ = state
				.Client
				.post(&Cb)
				.json(&Result)
				.timeout(Duration::from_secs(5))
				.send()
				.await;
		});
		return Json(ToolRelayResponse { Success:true, Result:None, error:None, AsyncCall:true }).into_response();
	}

	match ExecuteToolRelay(&state, &Request.Tool, &Request.Params).await {
		Ok(Value) => {
			state.ToolRelaySuccess.fetch_add(1, Ordering::Relaxed);
			Json(ToolRelayResponse { Success:true, Result:Some(Value), error:None, AsyncCall:false }).into_response()
		},
		Err(e) => {
			state.ToolRelayFailure.fetch_add(1, Ordering::Relaxed);
			Json(ToolRelayResponse { Success:false, Result:None, error:Some(e), AsyncCall:false }).into_response()
		},
	}
}
