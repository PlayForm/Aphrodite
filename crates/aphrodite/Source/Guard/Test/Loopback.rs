//! The loopback peer + `Host` decision, exercised end to end.
//!
//! Every case here calls `Guard::Loopback::Fn` - the exact function the
//! middleware calls - and covers every real caller in this repo plus the attack
//! the `Host` check exists to close. Each test is named for the one case it
//! covers; the module path names the decision.

use std::net::SocketAddr;

use crate::Guard::Loopback::Fn::Fn as Check;

/// A loopback IPv4 peer on `Port`.
fn Local(Port:u16) -> SocketAddr {
	use std::net::{Ipv4Addr, SocketAddrV4};
	SocketAddr::V4(SocketAddrV4::new(Ipv4Addr::LOCALHOST, Port))
}

/// A LAN (non-loopback) IPv4 peer on `Port`.
fn Lan(Port:u16) -> SocketAddr {
	use std::net::{Ipv4Addr, SocketAddrV4};
	SocketAddr::V4(SocketAddrV4::new(Ipv4Addr::new(192, 168, 1, 50), Port))
}

/// The Python plugin's health check and every documented curl/OpenAI-client
/// example in this repo address the proxy as `127.0.0.1:PORT`, and a standard
/// HTTP client sets `Host` to exactly that.
#[test]
fn Headers() {
	assert!(Check(Local(9797), "127.0.0.1:9797").is_ok());
	assert!(Check(Local(9798), "127.0.0.1:9798").is_ok());
	assert!(Check(Local(9797), "localhost:9797").is_ok());
	assert!(Check(Local(9797), "127.0.0.1").is_ok(), "no-port Host must also pass");
}

/// 02-F6: a missing/unparseable `Host` must be REJECTED, not waved through -
/// otherwise a client that can omit or strip `Host` (HTTP/1.0, some non-browser
/// `fetch` engines) skips the second defense layer entirely, leaving only the
/// peer-IP check the `Host` check exists to back up.
#[test]
fn Missing() {
	assert!(Check(Local(9797), "").is_err());
}

/// An IPv6 loopback peer presenting the bracketed literal.
#[test]
fn Ipv6() {
	use std::net::{Ipv6Addr, SocketAddrV6};
	let Address = SocketAddr::V6(SocketAddrV6::new(Ipv6Addr::LOCALHOST, 9797, 0, 0));
	assert!(Check(Address, "[::1]:9797").is_ok());
}

/// The scenario DNS rebinding produces: the TCP peer genuinely is loopback
/// (the attacker's hostname resolves to 127.0.0.1), but `Host` carries the
/// attacker's own domain.
#[test]
fn Rebinding() {
	let Result = Check(Local(9797), "attacker.example:9797");
	assert!(Result.is_err());
}

/// Pre-existing behavior must be unchanged: a non-loopback TCP peer is rejected
/// even when it presents a "correct-looking" `Host` header.
#[test]
fn Peer() {
	let Result = Check(Lan(9797), "127.0.0.1:9797");
	assert!(Result.is_err());
}

/// A non-loopback peer with no `Host` at all is rejected by the peer check.
#[test]
fn Peerless() {
	let Result = Check(Lan(9797), "");
	assert!(Result.is_err());
}
