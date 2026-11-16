//! `Host` header to hostname, with the port removed.

use std::borrow::Cow;

/// Strip an optional trailing `:port` from a `Host` header value - but not
/// from a bracketed IPv6 literal's own colons (`[::1]:9797` -> `[::1]`).
///
/// Borrowed in every case a real client produces: both branches are a prefix of
/// the header they were handed, so the loopback check that calls this allocates
/// nothing. The one owned case is a malformed bracketed value with no closing
/// `]` (`[::1`), where the only way to answer the same bytes as before is to
/// build the repaired form - and a `Cow` says exactly that instead of hiding a
/// `String` allocation on every request.
pub fn HostHeaderToHostname(Host:&str) -> Cow<'_, str> {
	match Host.strip_prefix('[') {
		Some(Rest) => {
			match Rest.find(']') {
				// `+ 2` is the opening bracket this match stripped and the
				// closing one just found, so the slice is the bracketed literal
				// itself.
				Some(Close) => Cow::Borrowed(&Host[..Close + 2]),
				None => Cow::Owned(format!("[{Rest}]")),
			}
		},
		None => Cow::Borrowed(Host.split(':').next().unwrap_or("")),
	}
}
