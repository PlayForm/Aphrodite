//! The rejection for an over-limit request body: 413, in the crate's JSON shape.
//!
//! Same status `DefaultBodyLimit` uses, and the same `{"error": ...}` body, so
//! a client cannot tell which of the two caps rejected it - both mean the same
//! thing.

use axum::{
	http::StatusCode,
	response::{IntoResponse, Json, Response},
};

/// Rejection for [`CappedBody`] - 413 with the crate's standard
/// `{"error": ...}` JSON shape (the same status `DefaultBodyLimit` uses).
#[derive(Debug)]
pub struct Struct;

impl IntoResponse for Struct {
	fn into_response(self) -> Response {
		(
			StatusCode::PAYLOAD_TOO_LARGE,
			Json(serde_json::json!({"error": "request body exceeds configured max_request_body_size limit"})),
		)
			.into_response()
	}
}
