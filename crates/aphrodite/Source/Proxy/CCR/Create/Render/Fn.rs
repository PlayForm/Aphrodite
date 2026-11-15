//! Render the create response.
//!
//! The wire shape is the contract: field names, the ratio, and the two sizes are
//! exactly what `CCR::Create::Response` serialises, and this is the only place
//! they are populated.

use axum::response::{IntoResponse, Json, Response};

use crate::Proxy::CCR::Create::{Outcome::Struct::Struct as CreateOutcome, Response::Struct::Struct as CreateResponse};

/// The success response for a stored entry.
pub(crate) fn Fn(Outcome:CreateOutcome) -> Response {
	let CompressedSize = Outcome.CompressedSize;

	Json(CreateResponse {
		TokenSavingsRatio:Outcome.TokenSavingsRatio(),
		OriginalSize:Outcome.OriginalSize,
		CompressedSize,
		MarkerSize:CompressedSize,
		hash:Outcome.Hash,
	})
	.into_response()
}
