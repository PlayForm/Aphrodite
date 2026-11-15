//! `Host` header parsing and the loopback allowlist.

use crate::Guard::{Host::Fn::HostHeaderToHostname, Loopback::Fn::AllowedLoopbackHosts};

// ── T5 (F1): Host-header validation must accept every loopback form a
// real client uses and reject a DNS-rebinding attacker's own hostname,
// which is where the peer-IP-only check (still enforced separately)
// cannot help - the TCP connection genuinely is loopback in that attack. ──
#[test]
fn TestHostHeaderToHostnameStripsPort() {
	assert_eq!(HostHeaderToHostname("127.0.0.1:9797"), "127.0.0.1");
	assert_eq!(HostHeaderToHostname("localhost:9798"), "localhost");
}

#[test]
fn TestHostHeaderToHostnameNoPort() {
	assert_eq!(HostHeaderToHostname("127.0.0.1"), "127.0.0.1");
	assert_eq!(HostHeaderToHostname("localhost"), "localhost");
}

#[test]
fn TestHostHeaderToHostnameIpv6Bracketed() {
	assert_eq!(HostHeaderToHostname("[::1]:9797"), "[::1]");
	assert_eq!(HostHeaderToHostname("[::1]"), "[::1]");
}

#[test]
fn TestAllowedLoopbackHostsAcceptsRealClients() {
	["127.0.0.1", "localhost", "[::1]"].into_iter().for_each(|h| {
		assert!(AllowedLoopbackHosts.contains(&h), "{h} must be an allowed loopback host");
	});
}

#[test]
fn TestAllowedLoopbackHostsRejectsDnsRebindingHostname() {
	// The exact attack this closes: a hostile page's JS does
	// `fetch("http://attacker.example:9797/retrieve")` where
	// `attacker.example` DNS-resolves to 127.0.0.1 - the TCP peer is
	// loopback (passes the IP check), but the Host header carries the
	// attacker's own domain, not a loopback name.
	let Hostname = HostHeaderToHostname("attacker.example:9797");
	assert!(!AllowedLoopbackHosts.contains(&Hostname.as_str()));
}
