//! Assembling the app: the public probe, the restricted groups, and the two
//! layers that make the restricted groups restricted.

use std::sync::Arc;

use aphrodite::Proxy::State::Struct::Struct as AppState;
use axum::{Router, extract::DefaultBodyLimit, middleware};
use Boot::Route::{CCR, Chat, Health, Management, Relay, Retrieve};
use Guard::{Bearer::Middleware::RequireMgmtToken, Loopback::Middleware::LoopbackOnly};

/// Build one listener's complete router.
///
/// The layer order is the security contract, so it is spelled out once, here:
/// `/health` is merged in last and outside both layers; everything else gets
/// `RequireMgmtToken` and then `LoopbackOnly`. The bearer gate is applied
/// before the catch-all route is merged, so it never guards the actual
/// LLM-proxying traffic - that traffic's credential is the upstream API key, not
/// this token.
///
/// There is deliberately no `CorsLayer` (report 12 F1): this proxy has no
/// legitimate browser-based consumer, so a permissive layer would only let any
/// website a user visits `fetch()` `/history`/`/retrieve`/`/ccr/*` cross-origin
/// and read or tamper with stored session content. `LoopbackOnly`'s peer-IP
/// check does not stop that, because the browser making the request genuinely is
/// a loopback client.
pub fn Build(State:Arc<AppState>) -> Router {
	let Restricted:Router<Arc<AppState>> = Router::new()
		.merge(Management::Route(&State))
		.merge(CCR::Route())
		.merge(Relay::Route())
		.merge(Retrieve::Route())
		.merge(Health::Upstream::Route(&State))
		// 1 MB body limit on management routes only (F7) - the catch-all
		// proxy route gets its own, much larger limit below: config
		// advertises max_context up to 1,000,000 tokens, and a chat request
		// at even 200k tokens is easily >1 MB of JSON with tool schemas and
		// history, so a blanket 1 MB cap on `/{*path}` rejected large agent
		// conversations with 413 before `ProxyHandler` ever ran.
		.layer(DefaultBodyLimit::max(1024 * 1024))
		// 02-F1: bearer-token gate on management routes only - applied
		// before merging with `CatchAll` below, so it never guards the
		// actual LLM-proxying `/{*path}` route (a separate concern; that
		// traffic's credential is the upstream API key, not this token).
		.layer(middleware::from_fn(RequireMgmtToken));

	// Catch-all proxy route: its own, much larger body limit.
	let CatchAll:Router<Arc<AppState>> = Chat::Route();

	let Restricted:Router<Arc<AppState>> = Restricted
		.merge(CatchAll)
		// Loopback enforcement layer on all non-/health routes
		.layer(middleware::from_fn(LoopbackOnly));

	Router::new().merge(Health::Public::Route()).merge(Restricted).with_state(State)
}
