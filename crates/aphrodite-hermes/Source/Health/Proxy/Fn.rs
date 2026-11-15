//! The TCP-connect proxy liveness probe.
//!
//! Reports what the proxies are DOING, not what the config says they should be
//! doing: a short connect_timeout against loopback is the only honest answer to
//! "is the engine actually serving this session". A dead proxy must surface as
//! `alive: false`, never as an error - `aphrodite_stats` and the session-init
//! debug line both consume this value and must keep reporting.

use crate::Health::Port::Fn::configured_ports;

/// Probe whether the cache and token proxies are listening.
pub(crate) fn proxy_health() -> serde_json::Value {
	use std::{net::TcpStream, time::Duration};
	let timeout = Duration::from_millis(400);
	let alive = |addr:String| {
		addr.parse()
			.ok()
			.and_then(|a| TcpStream::connect_timeout(&a, timeout).ok())
			.is_some()
	};
	let (cache_port, token_port) = configured_ports();
	serde_json::json!({
		"token": {"port": token_port, "alive": alive(format!("127.0.0.1:{token_port}"))},
		"cache": {"port": cache_port, "alive": alive(format!("127.0.0.1:{cache_port}"))},
	})
}
