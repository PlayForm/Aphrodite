//! Render a retrieval failure as the wire response.

use axum::{
	http::StatusCode,
	response::{IntoResponse, Json, Response},
};

use crate::Retrieve::Handle::Failure;

/// The status and body for one failure.
///
/// One match over the failure value decides all three response shapes, so the
/// status can never disagree with the body it is sent with. The two `Source`
/// spellings are load-bearing: a lookup failure reports `"none"` (nothing was
/// served) while a refused window reports `"ccr"` (the entry was found, the
/// window was not), and the refusal puts its message in `content` rather than
/// `error` because the message describes the request, not a missing entry.
pub(crate) fn Fn(Failure:&Failure::Enum::Enum) -> Response {
	let (Status, Source, Content, Error) = match Failure {
		Failure::Enum::Enum::Missing => (StatusCode::BAD_REQUEST, "none", None, Some("`hash` required".to_string())),
		Failure::Enum::Enum::Poisoned => {
			(
				StatusCode::INTERNAL_SERVER_ERROR,
				"none",
				None,
				Some("inline_ccr lock poisoned; cannot serve /retrieve".to_string()),
			)
		},
		Failure::Enum::Enum::Absent(hash) => {
			(
				StatusCode::NOT_FOUND,
				"none",
				None,
				Some(format!("CCR entry not found: {}", hash)),
			)
		},
		Failure::Enum::Enum::Refused(Error) => (StatusCode::BAD_REQUEST, "ccr", Some(format!("{Error:#}")), None),
	};

	(
		Status,
		Json(crate::Retrieve::Response::Struct::Struct {
			Found:false,
			Content,
			Source:Source.to_string(),
			error:Error,
			Truncated:false,
			Progress:None,
		}),
	)
		.into_response()
}
