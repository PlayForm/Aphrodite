//! The loopback allow/reject decision, with no axum types in sight.
//!
//! Kept free of `Request`/`Next` on purpose: the middleware around it is a
//! translation layer, so the rule stated here can be exercised by a unit test
//! with nothing but a `SocketAddr` and a header string.

use std::net::SocketAddr;

use Guard::Host::Fn::HostHeaderToHostname;

/// Hostnames a legitimate loopback caller can present in its `Host` header.
/// report 12 F1: a peer-IP check alone is not DNS-rebinding-safe - a
/// hostile webpage's JS can point a hostname it controls at 127.0.0.1 via
/// DNS, making the TCP connection genuinely loopback while the `Host`
/// header carries the attacker's own domain. Requiring `Host` to name the
/// loopback address itself closes that gap; real clients (Hermes, the
/// Python plugin, `curl`) already address the proxy this way.
pub const AllowedLoopbackHosts:&[&str] = &["localhost", "127.0.0.1", "[::1]", "::1"];

/// The actual allow/reject decision `LoopbackOnly` enforces, pulled out as
/// a pure function so it's directly unit-testable without constructing a
/// real axum `Request`/`Next` - the middleware below is a thin wrapper that
/// only translates this `Result` into an HTTP response.
pub fn CheckLoopbackRequest(Addr:SocketAddr, HostHeader:&str) -> Result<(), &'static str> {
	if !Addr.ip().is_loopback() {
		return Err("only loopback clients allowed");
	}
	// A missing/unparseable Host is rejected, not waved through: peer-IP
	// loopback is the first layer, but this second layer exists specifically
	// to catch a DNS-rebinding request that genuinely originates from
	// loopback yet carries a non-loopback (or absent) Host - HTTP/1.1
	// mandates Host, and every real caller here (curl, the Hermes plugin)
	// sends it, so there's no legitimate case to exempt (02-F6).
	let Hostname = HostHeaderToHostname(HostHeader);
	if !AllowedLoopbackHosts.contains(&Hostname.as_str()) {
		return Err("Host header does not name a loopback address");
	}
	Ok(())
}
