//! Who is asking.

use std::net::SocketAddr;

/// The peer a request came from, as the upstream probe cares about it.
///
/// The probe spends the operator's upstream rate-limit quota, so "is this the machine
/// itself" is not a boolean the handler tests: it is the fact that selects the answer,
/// and the two answers are different computations - one is a refusal that never touches
/// the network, the other probes the upstream and caches the result. A boolean could not
/// carry that: `match` on it is `clippy::match_bool`, and the two arms are futures of
/// different types, so no `map_or` unifies them.
pub(crate) enum Enum {
	/// The machine itself: the probe may run.
	Local,
	/// Any other peer: the answer is a refusal, and no request is made.
	Remote,
}

impl Enum {
	/// Classify a connected peer.
	pub(crate) fn Of(Addr:&SocketAddr) -> Self {
		Addr.ip().is_loopback().then_some(Self::Local).unwrap_or(Self::Remote)
	}
}
