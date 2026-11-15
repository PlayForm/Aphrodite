//! `Host` header to hostname, with the port removed.

/// Strip an optional trailing `:port` from a `Host` header value - but not
/// from a bracketed IPv6 literal's own colons (`[::1]:9797` -> `[::1]`).
pub fn HostHeaderToHostname(Host:&str) -> String {
	if let Some(Rest) = Host.strip_prefix('[') {
		Rest.split(']').next().map(|h| format!("[{h}]")).unwrap_or_default()
	} else {
		Host.split(':').next().unwrap_or("").to_string()
	}
}
