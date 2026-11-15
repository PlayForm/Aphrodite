//! The axum wrapper around [`CheckBearerToken`].

use axum::{
	extract::Request,
	http::StatusCode,
	middleware::Next,
	response::{IntoResponse, Json},
};

use crate::Guard::{CheckBearerToken, MgmtToken};

/// Bearer gate for the management routes.
///
/// `/metrics` is exempt by path: a Prometheus scraper typically cannot be
/// configured to send a bearer token, and the endpoint predates this layer with
/// an explicit "local-only deployments" note. `/health` never reaches this
/// middleware at all - it is merged in on a separate router with no layers, so
/// load-balancer probes work unauthenticated. Everything else on the restricted
/// router (`/stats`, `/stats/db`, `/history`, `/retrieve`, `/ccr/*`, `/reload`,
/// `/tool/relay`, `/version`, `/health/upstream`) is gated; `/stats` is
/// deliberately NOT exempt even though it exposes the same data as `/metrics`
/// plus request history.
pub async fn RequireMgmtToken(
	Request:Request,
	next:Next,
) -> Result<impl IntoResponse, (StatusCode, Json<serde_json::Value>)> {
	if Request.uri().path() == "/metrics" {
		return Ok(next.run(Request).await);
	}
	let Token = MgmtToken();
	let Auth = Request
		.headers()
		.get(axum::http::header::AUTHORIZATION)
		.and_then(|v| v.to_str().ok())
		.unwrap_or("");
	if let Err(Message) = CheckBearerToken(Token.as_deref(), Auth) {
		return Err((StatusCode::UNAUTHORIZED, Json(serde_json::json!({"error": Message}))));
	}
	Ok(next.run(Request).await)
}
