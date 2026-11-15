//! How an upstream call failed.
//!
//! `Other` is a category this proxy did not previously have. Until now every
//! non-timeout transport failure - connection refused, DNS failure, TLS error -
//! was counted as an upstream connect error, which made
//! `upstream_errors.connect_errors` a mix of two unrelated diagnoses. Splitting
//! them changes metric classification, so it is a behaviour change and is named
//! as one in the handoff.

/// The failure class.
pub(crate) enum Enum {
	/// The request timed out. A post-send timeout may already have been
	/// accepted upstream, which is why it never retries.
	Timeout,
	/// The connection was never established, so nothing was sent.
	Connect,
	/// Any other transport failure - DNS, TLS, a malformed URL, a body error.
	Other,
}
