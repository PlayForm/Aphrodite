//! Reading `APHRODITE_MGMT_TOKEN` - the one credential the management routes
//! check.

/// Environment contract: `APHRODITE_MGMT_TOKEN`, read fresh on every request;
/// `None` means unset or empty, which disables the bearer gate entirely.
///
/// 02-F1: `Loopback::Middleware::Fn`'s peer-IP + Host check stops a remote peer and a
/// DNS-rebinding *read* - but a hostile local page can still issue a CORS
/// "simple request" write (`fetch(..., {method:"POST", mode:"no-cors"})`
/// never preflights, and the browser genuinely is a loopback client with a
/// valid Host header). A bearer token checked on the management routes
/// (`/stats`, `/history`, `/retrieve`, `/ccr/*`, `/reload`, `/tool/relay`,
/// `/version`, `/stats/db`, `/health/upstream`) closes that: seeding CCR
/// entries, evicting a victim's markers via `/reload`, or reading stored
/// session content via `/retrieve`/`/history` all require it now.
///
/// `/health` is a separate router merged in without this layer at all (load
/// balancer probes need to reach it with no credentials). `/metrics` is
/// exempt by path check below - per 02-F11, a Prometheus scraper typically
/// can't be configured to send a bearer token, and this endpoint predates
/// this layer with an explicit "local-only deployments" comment; `/stats`
/// (which exposes the same data plus request history) is NOT exempt.
///
/// Back-compat default: if `APHRODITE_MGMT_TOKEN` is unset, every request
/// passes (unchanged behavior) - `run_single` logs one startup `warn!` so an
/// operator relying on that default isn't silently unprotected.
pub fn Fn() -> Option<String> { std::env::var("APHRODITE_MGMT_TOKEN").ok().filter(|Token| !Token.is_empty()) }
