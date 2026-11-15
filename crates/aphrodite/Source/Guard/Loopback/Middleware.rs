//! The axum wrapper around [`CheckLoopbackRequest`].
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

use crate::Guard::Loopback::Fn::CheckLoopbackRequest;

/// Middleware that rejects non-loopback clients on all routes except /health.
/// /health is intentionally exempt so external load-balancer probes work.
pub async fn LoopbackOnly(
	ConnectInfo(Addr):ConnectInfo<SocketAddr>,
	Request:Request,
	next:Next,
) -> Result<impl IntoResponse, (StatusCode, Json<serde_json::Value>)> {
	let Host = Request
		.headers()
		.get(axum::http::header::HOST)
		.and_then(|v| v.to_str().ok())
		.unwrap_or("");
	if let Err(Message) = CheckLoopbackRequest(Addr, Host) {
		return Err((StatusCode::FORBIDDEN, Json(serde_json::json!({"error": Message}))));
	}
	Ok(next.run(Request).await)
}
