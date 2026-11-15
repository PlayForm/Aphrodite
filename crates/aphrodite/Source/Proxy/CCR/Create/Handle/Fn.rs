//! `POST /ccr/create` - decode, command, execute, report, notify.
//!
//! Stores through the same store rules the proxy uses, so an externally created
//! entry behaves exactly like an intercepted one. The handler is the sequence its
//! stage comments named, with no early returns between the stages: each stage
//! answers with a value, and both failures render through the same rejection
//! path.
//!
//! The ratio sample and the webhook are taken off the outcome with `mem::take`
//! before rendering, which is how a raw-body create - whose command carried
//! neither - simply does nothing in both places.

use std::sync::Arc;

use axum::{extract::State, response::Response};
use bytes::Bytes;

use crate::Proxy::{
	CCR::{
		Create::{
			Command::Build::Fn::Fn as BuildCommand,
			Decode::Fn::Fn as DecodeBody,
			Execute::Fn::Fn as ExecuteCreate,
			Rejection::Render::Fn::Fn as RenderRejection,
			Render::Fn::Fn as RenderCreate,
		},
		Notification::Schedule::Fn::Fn as ScheduleNotification,
	},
	State::Struct::Struct as AppState,
};

/// `POST /ccr/create` - store content directly into the CCR backend.
pub async fn Fn(State(state):State<Arc<AppState>>, Headers:axum::http::HeaderMap, body:Bytes) -> Response {
	let ContentType = Headers.get("content-type").and_then(|Value| Value.to_str().ok()).unwrap_or("");
	let Decoded = match DecodeBody(ContentType, &body) {
		Ok(Decoded) => Decoded,
		Err(Rejection) => return RenderRejection(Rejection),
	};
	let Command = BuildCommand(Decoded, state.NotifyURL.as_deref());

	match ExecuteCreate(&state, Command).await {
		Ok(mut Outcome) => {
			// The compression-ratio EMA reflects how compressible content
			// flowing through `/ccr/create` is, so `/stats` is not blind to this
			// path. A raw-body create carries no sample and records none.
			std::mem::take(&mut Outcome.Estimate)
				.into_iter()
				.for_each(|Estimate| state.UpdateCompressionRatio(Outcome.OriginalSize, Estimate));
			std::mem::take(&mut Outcome.Notification)
				.into_iter()
				.for_each(|Event| ScheduleNotification(&state, Event));

			RenderCreate(Outcome)
		},
		Err(Rejection) => RenderRejection(Rejection),
	}
}
