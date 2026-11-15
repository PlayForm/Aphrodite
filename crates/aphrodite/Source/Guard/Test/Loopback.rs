//! The loopback peer + `Host` decision, exercised end to end.
//!
//! Every case here calls `CheckLoopbackRequest` - the exact function the
//! middleware calls - and covers every real caller in this repo plus the attack
//! the `Host` check exists to close.

use std::net::SocketAddr;

use crate::Guard::CheckLoopbackRequest;

fn LoopbackV4(Port:u16) -> SocketAddr {
	use std::net::{Ipv4Addr, SocketAddrV4};
	SocketAddr::V4(SocketAddrV4::new(Ipv4Addr::LOCALHOST, Port))
}

fn LanV4(Port:u16) -> SocketAddr {
	use std::net::{Ipv4Addr, SocketAddrV4};
	SocketAddr::V4(SocketAddrV4::new(Ipv4Addr::new(192, 168, 1, 50), Port))
}

#[test]
fn TestCheckLoopbackRequestAllowsRealClientHostHeaders() {
	// The Python plugin's health check and every documented curl/OpenAI-
	// client example in this repo address the proxy as `127.0.0.1:PORT`;
	// a standard HTTP client sets `Host` to exactly that.
	assert!(CheckLoopbackRequest(LoopbackV4(9797), "127.0.0.1:9797").is_ok());
	assert!(CheckLoopbackRequest(LoopbackV4(9798), "127.0.0.1:9798").is_ok());
	assert!(CheckLoopbackRequest(LoopbackV4(9797), "localhost:9797").is_ok());
	assert!(
		CheckLoopbackRequest(LoopbackV4(9797), "127.0.0.1").is_ok(),
		"no-port Host must also pass"
	);
}

// 02-F6: a missing/unparseable Host must be REJECTED, not waved through -
// otherwise a client that can omit or strip Host (HTTP/1.0, some
// non-browser `fetch` engines) skips the second defense layer entirely,
// leaving only the peer-IP check the Host check exists to back up.
#[test]
fn TestCheckLoopbackRequestRejectsMissingHostHeader() {
	assert!(CheckLoopbackRequest(LoopbackV4(9797), "").is_err());
}

#[test]
fn TestCheckLoopbackRequestAllowsIpv6Loopback() {
	use std::net::{Ipv6Addr, SocketAddrV6};
	let Addr = SocketAddr::V6(SocketAddrV6::new(Ipv6Addr::LOCALHOST, 9797, 0, 0));
	assert!(CheckLoopbackRequest(Addr, "[::1]:9797").is_ok());
}

#[test]
fn TestCheckLoopbackRequestRejectsDnsRebinding() {
	// TCP peer genuinely is loopback (this is the scenario DNS
	// rebinding produces - the attacker's hostname resolves to
	// 127.0.0.1), but Host carries the attacker's own domain.
	let Result = CheckLoopbackRequest(LoopbackV4(9797), "attacker.example:9797");
	assert!(Result.is_err());
}

#[test]
fn TestCheckLoopbackRequestRejectsNonLoopbackPeerRegardlessOfHost() {
	// Pre-existing behavior must be unchanged: a non-loopback TCP peer
	// is rejected even when it presents a "correct-looking" Host header.
	let Result = CheckLoopbackRequest(LanV4(9797), "127.0.0.1:9797");
	assert!(Result.is_err());
}

#[test]
fn TestCheckLoopbackRequestRejectsNonLoopbackPeerWithNoHost() {
	let Result = CheckLoopbackRequest(LanV4(9797), "");
	assert!(Result.is_err());
}
