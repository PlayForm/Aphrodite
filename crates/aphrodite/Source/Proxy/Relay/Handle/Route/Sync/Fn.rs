//! Execute the tool inline and answer with its result.
//!
//! Both relay counters are incremented here, from the result's own shape, so
//! `tool_relay_success + tool_relay_failure` matches `tool_relay_calls` on this
//! path.

use std::sync::{Arc, atomic::Ordering};

use axum::response::{IntoResponse, Json, Response};

use crate::Proxy::{
	Relay::{Execute::Fn::Fn as ExecuteToolRelay, Response::Struct::Struct as ToolRelayResponse},
	State::Struct::Struct as AppState,
};

/// Run the tool and render its answer.
pub(crate) async fn Fn(state:&Arc<AppState>, Tool:&str, Params:&serde_json::Value) -> Response {
	match ExecuteToolRelay(state, Tool, Params).await {
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
