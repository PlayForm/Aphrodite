//! Decode the request body.
//!
//! The content type decides which shape is attempted, and a failure to match it
//! is a rejection rather than a fallback: a body that said `application/json`
//! and is not JSON gets a 400 naming the parse error, and a body that is not
//! UTF-8 gets a 400 saying so, because neither can be stored as content the
//! model could later read.

use bytes::Bytes;

use crate::Proxy::CCR::Create::{
	Decode::Enum::Enum as DecodedBody,
	Rejection::Enum::Enum as CreateRejection,
	Request::Struct::Struct as CreateRequest,
};

/// Decode the body according to its declared content type.
pub(crate) fn Fn(ContentType:&str, Body:&Bytes) -> Result<DecodedBody, CreateRejection> {
	if ContentType.contains("json") {
		serde_json::from_slice::<CreateRequest>(Body)
			.map(DecodedBody::Json)
			.map_err(|e| CreateRejection::InvalidJson(format!("invalid JSON: {e}")))
	} else {
		String::from_utf8(Body.to_vec())
			.map(DecodedBody::Opaque)
			.map_err(|_| CreateRejection::InvalidUtf8)
	}
}
