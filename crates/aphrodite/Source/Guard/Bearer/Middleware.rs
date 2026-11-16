//! The axum wrapper around [`Fn`](crate::Guard::Bearer::Fn::Fn).

use axum::{
	extract::Request,
	http::StatusCode,
	middleware::Next,
	response::{IntoResponse, Json},
};

use crate::Guard::{Bearer::Fn::Fn as Bearer, Token::Fn::Fn as Token};

/// Bearer gate for the management routes: applied by `Boot::Route::Build` to the
/// restricted router, and the only place `APHRODITE_MGMT_TOKEN` is enforced.
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
pub async fn Fn(Request:Request, Next:Next) -> Result<impl IntoResponse, (StatusCode, Json<serde_json::Value>)> {
	// The gate is the token check itself, or nothing at all when the path is
	// exempt - so `Token` is read only on a gated route, exactly as before.
	let Gate = (Request.uri().path() != "/metrics").then(|| {
		let Configured = Token();
		let Header = Request
			.headers()
			.get(axum::http::header::AUTHORIZATION)
			.and_then(|Value| Value.to_str().ok())
			.unwrap_or("");

		Bearer(Configured.as_deref(), Header)
	});

	match Gate {
		Some(Err(Message)) => {
			Err((
				StatusCode::UNAUTHORIZED,
				Json(serde_json::json!({"error": format!("{Message:#}")})),
			))
		},
		Some(Ok(())) | None => Ok(Next.run(Request).await),
	}
}
