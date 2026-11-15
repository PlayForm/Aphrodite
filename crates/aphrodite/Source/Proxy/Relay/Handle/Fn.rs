//! The `/tool_relay` handler: validate, dispatch, answer.
//!
//! Entry point only: it opens the relay span and hands the request to `Route`.
//! The span is entered through `Instrument` rather than an `Entered` guard -
//! this is an async fn, and a guard held across an await point would attribute
//! another task's events to this call.

use std::sync::Arc;

use axum::{
	extract::State,
	response::{IntoResponse, Json},
};
use tracing::Instrument;

use super::Route::Fn::Fn as Route;
use crate::Proxy::{Relay::Request::Struct::Struct as ToolRelayRequest, State::Struct::Struct as AppState};

/// `POST /tool_relay` - dispatches a Hermes-side tool call (see
/// [`crate::Proxy::Relay::Execute::Fn::Fn`]). Runs synchronously unless the
/// request carries an `https://` `CallbackURL`, in which case it is spawned onto
/// `TaskTracker` and the result is POSTed back later instead of returned inline.
pub async fn Fn(State(state):State<Arc<AppState>>, Json(Request):Json<ToolRelayRequest>) -> impl IntoResponse {
	// The tool name identifies the work; `callback` says which answer path will
	// be taken. Neither the parameters nor the result enters the span - a tool
	// call carries content and can carry a credential.
	let Span = tracing::info_span!(
		"proxy.tool_relay",
		tool = %Request.Tool,
		callback = Request.CallbackURL.is_some(),
	);
	Route(state, Request).instrument(Span).await
}
