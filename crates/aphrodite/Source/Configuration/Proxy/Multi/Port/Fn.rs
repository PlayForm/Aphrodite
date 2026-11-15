//! The per-mode port overrides that let several proxies coexist on one
//! machine.

use std::net::SocketAddr;

use super::super::Struct::Struct;

impl Struct {
	/// Override `Listen`'s port from the named env var, if set.
	///
	/// A missing env var is the common case and silently keeps `Listen`
	/// unchanged. A *present but malformed* value (non-numeric, or outside
	/// the u16 port range) also keeps `Listen` unchanged, but logs a
	/// warning - silently ignoring a typo'd override left the operator with
	/// no way to tell "my override didn't apply" from "I didn't set an
	/// override", the same silent-failure class as the missing-CCR-
	/// directory bug this override was added alongside.
	///
	/// This is per-mode, never process-wide: `APHRODITE_CACHE_PORT` and
	/// `APHRODITE_TOKEN_PORT` exist precisely so a blanket
	/// `APHRODITE_LISTEN` cannot collapse the two-proxy split.
	pub(crate) fn ApplyPortOverride(Listen:SocketAddr, EnvironmentVar:&str) -> SocketAddr {
		match std::env::var(EnvironmentVar) {
			Ok(p) => {
				match p.parse::<u16>() {
					Ok(Port) => {
						let mut Addr = Listen;
						Addr.set_port(Port);
						tracing::info!("{}={} overriding listen to {}", EnvironmentVar, Port, Addr);
						Addr
					},
					Err(_) => {
						tracing::warn!(
							"{}={:?} is not a valid port (1-65535); ignoring override, using {}",
							EnvironmentVar,
							p,
							Listen,
						);
						Listen
					},
				}
			},
			Err(_) => Listen,
		}
	}
}
