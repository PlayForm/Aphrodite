//! Extract the upstream head.

use axum::http::HeaderMap;

use crate::Proxy::Upstream::Head::Struct::Struct as UpstreamHead;

/// Capture the status and the header map before the body is consumed.
///
/// F5: the whole header map, not just `content-type` - the proxy used to
/// rebuild the response with only `Content-Type`, silently dropping
/// `Retry-After`, `x-ratelimit-*`, `request-id`, etc. that clients rely on for
/// backoff and support.
pub(crate) fn Fn(Response:&reqwest::Response) -> UpstreamHead {
	let Headers:HeaderMap = Response.headers().clone();

	UpstreamHead {
		Status:Response.status(),
		ContentType:Headers.get("content-type").cloned(),
		Headers,
	}
}
