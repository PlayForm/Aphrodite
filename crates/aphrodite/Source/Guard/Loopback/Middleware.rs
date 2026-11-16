//! The axum wrapper around [`Fn`](crate::Guard::Loopback::Fn::Fn).
//!
//! It reads the peer address and the `Host` header, asks the decision function,
//! and turns a rejection into 403. It must stay this thin - any rule added here
//! is a rule the unit tests cannot see.

use std::net::SocketAddr;

use axum::{
	extract::{ConnectInfo, Request},
	http::StatusCode,
	middleware::Next,
	response::{IntoResponse, Json},
};

use crate::Guard::Loopback::Fn::Fn as Loopback;

/// Middleware that rejects non-loopback clients on all routes except /health.
/// /health is intentionally exempt so external load-balancer probes work.
///
/// External contract: applied by `Boot::Route::Build` to every router except the
/// `/health` one; a rejection is a 403 carrying `{"error": ...}`.
pub async fn Fn(
	ConnectInfo(Address):ConnectInfo<SocketAddr>,
	Request:Request,
	Next:Next,
) -> Result<impl IntoResponse, (StatusCode, Json<serde_json::Value>)> {
	let Header = Request
		.headers()
		.get(axum::http::header::HOST)
		.and_then(|Value| Value.to_str().ok())
		.unwrap_or("");
	match Loopback(Address, Header) {
		Ok(()) => Ok(Next.run(Request).await),
		Err(Message) => {
			Err((
				StatusCode::FORBIDDEN,
				Json(serde_json::json!({"error": format!("{Message:#}")})),
			))
		},
	}
}
