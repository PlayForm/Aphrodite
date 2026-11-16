//! The loopback allow/reject decision, with no axum types in sight.
//!
//! Kept free of `Request`/`Next` on purpose: the middleware around it is a
//! translation layer, so the rule stated here can be exercised by a unit test
//! with nothing but a `SocketAddr` and a header string.

use std::net::SocketAddr;

use super::Allowed::Const::Allowed;
use crate::Guard::Host::Fn::Fn as Hostname;

/// The actual allow/reject decision `Loopback::Middleware::Fn` enforces, pulled
/// out as a pure function so it's directly unit-testable without constructing a
/// real axum `Request`/`Next` - the middleware below is a thin wrapper that
/// only translates this `Result` into an HTTP response.
///
/// Two checks, both required: the peer address must be loopback, and the `Host`
/// header must name a loopback address. The accepted hostnames are
/// `Loopback::Allowed::Const`.
pub fn Fn(Address:SocketAddr, Header:&str) -> anyhow::Result<()> {
	anyhow::ensure!(Address.ip().is_loopback(), "only loopback clients allowed");
	// A missing/unparseable Host is rejected, not waved through: peer-IP
	// loopback is the first layer, but this second layer exists specifically
	// to catch a DNS-rebinding request that genuinely originates from
	// loopback yet carries a non-loopback (or absent) Host - HTTP/1.1
	// mandates Host, and every real caller here (curl, the Hermes plugin)
	// sends it, so there's no legitimate case to exempt (02-F6).
	let Hostname = Hostname(Header);
	anyhow::ensure!(
		Allowed.contains(&Hostname.as_ref()),
		"Host header does not name a loopback address"
	);
	Ok(())
}
