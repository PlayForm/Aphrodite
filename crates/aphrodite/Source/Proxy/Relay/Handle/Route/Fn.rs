//! The `/tool_relay` handler's body: validate, dispatch, answer.
//!
//! Split from `Fn` only so the relay span can be entered with `Instrument`.
//! Synchronous calls run inline and return their result; a request carrying a
//! `CallbackURL` is spawned onto the task tracker and answered immediately, so
//! the caller is never held open by a slow tool. Both paths count into the
//! relay counters.

use std::{
	sync::{Arc, atomic::Ordering},
	time::Duration,
};

use axum::{
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

pub(crate) async fn Fn(state:Arc<AppState>, Request:ToolRelayRequest) -> impl IntoResponse {
	state.ToolRelayCalls.fetch_add(1, Ordering::Relaxed);
	tracing::info!(Tool = %Request.Tool, "tool_relay");

	// Validate aphrodite_retrieve: requests with only `Query` and no `hash` are
	// invalid and must return 400 BAD_REQUEST instead of silently passing through.
	match Request.Tool == "aphrodite_retrieve" && Request.Params.get("hash").and_then(|v| v.as_str()).is_none() {
		true => {
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
		},
		false => {},
	}

	match &Request.CallbackURL {
		Some(Cb) => {
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
				// The callback body is the same `Result<Value, String>` shape the
				// synchronous path returns, so an `{"Ok": ...}` / `{"Err": "..."}`
				// envelope reaches the callback URL either way; only the error's
				// representation changes, and `{e:#}` keeps the whole anyhow chain.
				let Result = ExecuteToolRelay(&state, &Tool, &Params).await.map_err(|e| format!("{e:#}"));
				// F13: the execute result itself (success/failure of the tool
				// call) was never counted on this async path - only the
				// synchronous path below incremented these, so
				// `tool_relay_success + tool_relay_failure` silently diverged
				// from `ToolRelayCalls` for every async callback request.
				match Result.is_ok() {
					true => {
						state.ToolRelaySuccess.fetch_add(1, Ordering::Relaxed);
					},
					false => {
						state.ToolRelayFailure.fetch_add(1, Ordering::Relaxed);
					},
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
		},
		None => {},
	}

	match ExecuteToolRelay(&state, &Request.Tool, &Request.Params).await {
		Ok(Value) => {
			state.ToolRelaySuccess.fetch_add(1, Ordering::Relaxed);
			Json(ToolRelayResponse { Success:true, Result:Some(Value), error:None, AsyncCall:false }).into_response()
		},
		Err(e) => {
			state.ToolRelayFailure.fetch_add(1, Ordering::Relaxed);
			Json(ToolRelayResponse { Success:false, Result:None, error:Some(format!("{e:#}")), AsyncCall:false })
				.into_response()
		},
	}
}
