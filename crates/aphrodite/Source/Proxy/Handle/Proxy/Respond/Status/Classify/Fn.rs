//! Classify an upstream status.

use axum::http::StatusCode;

use crate::Proxy::Handle::Proxy::Respond::Status::Enum::Enum as ResponseStatus;

/// Which counter class this status belongs to.
pub(crate) fn Fn(Status:StatusCode) -> ResponseStatus {
	let Code = Status.as_u16();

	(Code >= 500)
		.then_some(ResponseStatus::ServerError)
		.or_else(|| (Code >= 400).then_some(ResponseStatus::ClientError))
		.unwrap_or(ResponseStatus::Other)
}
