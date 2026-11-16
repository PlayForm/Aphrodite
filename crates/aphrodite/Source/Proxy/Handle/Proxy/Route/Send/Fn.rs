//! The upstream road: build the URL, forward, respond.
//!
//! One function because a cache bypass and a cache miss take exactly the same
//! road - the difference is already recorded by the lookup, and duplicating the
//! forward to keep them apart would be two copies of one failure path.

use axum::response::Response;
use bytes::Bytes;

use crate::Proxy::{
	Handle::Proxy::{
		Forward::Fn::Fn as ForwardUpstream,
		Respond::Fn::Fn as RespondUpstream,
		Route::Url::Fn::Fn as Url,
	},
	Request::Context::Struct::Struct as RequestContext,
};

/// Send the request upstream and render the answer.
pub(crate) async fn Fn(Context:&RequestContext<'_>, Path:&axum::extract::OriginalUri, Body:Bytes) -> Response {
	let Target = Url(Context.State, Path);
	let Forwarded = ForwardUpstream(Context, &Target, Body).await;

	RespondUpstream(Context, Forwarded).await
}
