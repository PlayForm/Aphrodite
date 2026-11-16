//! `Host` header to hostname, with the port removed.

use std::borrow::Cow;

/// Strip an optional trailing `:port` from a `Host` header value - but not
/// from a bracketed IPv6 literal's own colons (`[::1]:9797` -> `[::1]`).
///
/// External contract: the input is the HTTP `Host` header value exactly as the
/// client sent it, and the output is the hostname `Loopback::Allowed::Const`
/// matches against - so `localhost:9797`, `127.0.0.1` and `[::1]:9797` all have
/// to reduce to a form on that list, and anything else is a rejection.
///
/// Borrowed in every case a real client produces: both branches are a prefix of
/// the header they were handed, so the loopback check that calls this allocates
/// nothing. The one owned case is a malformed bracketed value with no closing
/// `]` (`[::1`), where the only way to answer the same bytes as before is to
/// build the repaired form - and a `Cow` says exactly that instead of hiding a
/// `String` allocation on every request.
pub fn Fn(Header:&str) -> Cow<'_, str> {
	match Header.strip_prefix('[') {
		Some(Rest) => {
			match Rest.find(']') {
				// `+ 2` is the opening bracket this match stripped and the
				// closing one just found, so the slice is the bracketed literal
				// itself.
				Some(Close) => Cow::Borrowed(&Header[..Close + 2]),
				None => Cow::Owned(format!("[{Rest}]")),
			}
		},
		None => Cow::Borrowed(Header.split(':').next().unwrap_or("")),
	}
}
