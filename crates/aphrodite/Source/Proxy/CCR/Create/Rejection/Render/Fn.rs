//! One rejection, one status, one JSON body.
//!
//! F14: this endpoint used to answer with a hash and a savings ratio even when
//! `state.ccr` was `None`, so nothing was ever stored and every later
//! `/retrieve` of that hash 404s. Unavailability and a failed write are now
//! distinct statuses, and the messages are the ones the endpoint already
//! documented.

use axum::{
	http::StatusCode,
	response::{IntoResponse, Json, Response},
};

use crate::Proxy::CCR::Create::Rejection::Enum::Enum as CreateRejection;

/// Turn a rejection into the response the caller sees.
pub(crate) fn Fn(Rejection:CreateRejection) -> Response {
	match Rejection {
		CreateRejection::NotEnabled => {
			(
				StatusCode::SERVICE_UNAVAILABLE,
				Json(serde_json::json!({"error": "CCR not enabled"})),
			)
				.into_response()
		},
		CreateRejection::StoreFailed => {
			(
				StatusCode::INTERNAL_SERVER_ERROR,
				Json(serde_json::json!({"error": "failed to store content in CCR backend"})),
			)
				.into_response()
		},
		CreateRejection::InvalidJson(Detail) => {
			(StatusCode::BAD_REQUEST, Json(serde_json::json!({"error": Detail}))).into_response()
		},
		CreateRejection::InvalidUtf8 => {
			(
				StatusCode::BAD_REQUEST,
				Json(serde_json::json!({"error": "invalid UTF-8 in body"})),
			)
				.into_response()
		},
	}
}
