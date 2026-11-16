//! `Host` header parsing and the loopback allowlist.

use crate::Guard::{Host::Fn::Fn as Hostname, Loopback::Allowed::Const::Allowed};

// ── T5 (F1): Host-header validation must accept every loopback form a
// real client uses and reject a DNS-rebinding attacker's own hostname,
// which is where the peer-IP-only check (still enforced separately)
// cannot help - the TCP connection genuinely is loopback in that attack. ──

/// A trailing `:port` is removed.
#[test]
fn Port() {
	assert_eq!(&*Hostname("127.0.0.1:9797"), "127.0.0.1");
	assert_eq!(&*Hostname("localhost:9798"), "localhost");
}

/// A header with no port is answered unchanged.
#[test]
fn Bare() {
	assert_eq!(&*Hostname("127.0.0.1"), "127.0.0.1");
	assert_eq!(&*Hostname("localhost"), "localhost");
}

/// A bracketed IPv6 literal keeps its own colons and loses the port.
#[test]
fn Bracketed() {
	assert_eq!(&*Hostname("[::1]:9797"), "[::1]");
	assert_eq!(&*Hostname("[::1]"), "[::1]");
}

/// The one path that owns: a bracketed value with no closing `]` is answered
/// with the repaired form, which is what the `String`-returning version
/// produced for every input.
#[test]
fn Repair() {
	assert_eq!(&*Hostname("[::1"), "[::1]");
}

/// Every loopback form a real client presents is on the allowlist.
#[test]
fn Clients() {
	["127.0.0.1", "localhost", "[::1]"].into_iter().for_each(|Host| {
		assert!(Allowed.contains(&Host), "{Host} must be an allowed loopback host");
	});
}

/// The exact attack the allowlist closes: a hostile page's JS does
/// `fetch("http://attacker.example:9797/retrieve")` where `attacker.example`
/// DNS-resolves to 127.0.0.1 - the TCP peer is loopback (passes the IP check),
/// but the `Host` header carries the attacker's own domain, not a loopback name.
#[test]
fn Rebinding() {
	let Host = Hostname("attacker.example:9797");
	assert!(!Allowed.contains(&Host.as_ref()));
}
