//! Extract the upstream head, then branch exactly once.
//!
//! The status counters are recorded here because this is the only place both
//! interpreters pass through with the status in hand. The delivery branch is
//! the whole point of the split: one side must never buffer a stream and the
//! other must never stream a buffered body, so the two are separate functions
//! with separate accounting rather than two arms of a large `match`.

use std::sync::atomic::Ordering;

use axum::response::Response;

use crate::Proxy::{
	Handle::Proxy::Respond::{
		Buffered::Fn::Fn as RespondBuffered,
		Status::{Classify::Fn::Fn as ClassifyStatus, Enum::Enum as ResponseStatus},
		Stream::Fn::Fn as RespondStream,
	},
	Request::Context::Struct::Struct as RequestContext,
	Upstream::{
		Head::Fn::Fn as ExtractHead,
		Media::{Enum::Enum as UpstreamMedia, Fn::Fn as ClassifyMedia},
	},
};

/// Record the status class, then hand the response to its interpreter.
pub(crate) async fn Fn(Context:&RequestContext<'_>, Response:reqwest::Response) -> Response {
	let Head = ExtractHead(&Response);

	match ClassifyStatus(Head.Status) {
		ResponseStatus::ServerError => {
			Context.State.UpstreamErrors5xx.fetch_add(1, Ordering::Relaxed);
		},
		ResponseStatus::ClientError => {
			Context.State.UpstreamErrors4xx.fetch_add(1, Ordering::Relaxed);
		},
		ResponseStatus::Other => {},
	};

	match ClassifyMedia(Head.ContentType.as_ref()) {
		UpstreamMedia::EventStream => RespondStream(Context, Head, Response),
		UpstreamMedia::Buffered => RespondBuffered(Context, Head, Response).await,
	}
}
