//! Request-body extractor enforcing the configured `MaximumRequestBodySize`.
//!
//! Reads through `axum::body::to_bytes` with the configured cap, so an
//! over-limit body is rejected with 413 *while* it is being read - never fully
//! buffered into memory. `None` (the default) reads the whole body with a
//! `usize::MAX` cap, which is byte-identical to the unbounded behaviour it
//! replaced. Complements the router's fixed `DefaultBodyLimit`: this one is
//! config-driven.

use std::sync::Arc;

use axum::{
	body::to_bytes,
	extract::{FromRequest, Request},
};
use bytes::Bytes;

use crate::Proxy::{Body::TooLarge::Struct::Struct as BodyTooLarge, State::Struct::Struct as AppState};

/// Request-body extractor enforcing [`AppState::max_request_body_size`].
///
/// Wraps the `Bytes` extractor but reads the body through
/// `axum::body::to_bytes` with the configured cap, so an over-limit body
/// is rejected with 413 WHILE it is being read - never fully buffered into
/// memory. `None` (the default) reads the whole body with a `usize::MAX`
/// cap: byte-identical current behavior. Complements the router's
/// `DefaultBodyLimit` (a fixed 64 MB on the catch-all route): this is the
/// config-driven, per-`MaximumRequestBodySize` cap.
#[derive(Debug)]
pub struct Struct(pub Bytes);

impl FromRequest<Arc<AppState>> for Struct {
	type Rejection = BodyTooLarge;

	async fn from_request(Request:Request, State:&Arc<AppState>) -> Result<Self, Self::Rejection> {
		// Only the body stream is needed; the (already-extracted) parts are
		// dropped, exactly as the `Bytes` extractor does.
		let (_Parts, body) = Request.into_parts();
		let Cap = State.MaximumRequestBodySize.unwrap_or(usize::MAX);
		// `to_bytes` stops reading at `Cap` and errors on overflow, so the
		// cap is enforced during the read, never after a full buffering.
		match to_bytes(body, Cap).await {
			Ok(bytes) => Ok(Struct(bytes)),
			// Over-limit OR a body read failure - both degrade to the same
			// 413: a client that sent too much gets the same answer whether
			// the stream errored or was merely too long.
			Err(_) => Err(BodyTooLarge),
		}
	}
}
