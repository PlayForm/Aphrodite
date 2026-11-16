//! The loopback allowlist.

/// Hostnames a legitimate loopback caller can present in its `Host` header.
///
/// report 12 F1: a peer-IP check alone is not DNS-rebinding-safe - a
/// hostile webpage's JS can point a hostname it controls at 127.0.0.1 via
/// DNS, making the TCP connection genuinely loopback while the `Host`
/// header carries the attacker's own domain. Requiring `Host` to name the
/// loopback address itself closes that gap; real clients (Hermes, the
/// Python plugin, `curl`) already address the proxy this way.
///
/// External contract: these four literals are the complete set of accepted
/// `Host` header values, matched after the port is stripped. An absent or
/// unparseable header matches nothing and is therefore rejected.
pub const Allowed:&[&str] = &["localhost", "127.0.0.1", "[::1]", "::1"];
