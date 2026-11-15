//! Turn the upstream outcome into the client response.

use axum::response::Response;

use crate::Proxy::{
	Handle::Proxy::{
		Forward::Failure::Struct::Struct as FailureStruct,
		Respond::{Failure::Fn::Fn as RespondFailure, Upstream::Fn::Fn as RespondUpstream},
	},
	Request::Context::Struct::Struct as RequestContext,
};

/// Render the client response from the upstream outcome.
pub(crate) async fn Fn(Context:&RequestContext<'_>, Upstream:Result<reqwest::Response, FailureStruct>) -> Response {
	match Upstream {
		Ok(Response) => RespondUpstream(Context, Response).await,
		Err(Failure) => RespondFailure(Context, Failure),
	}
}
