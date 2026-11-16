//! The TCP-connect proxy liveness probe.
//!
//! Reports what the proxies are DOING, not what the config says they should be
//! doing: a short connect_timeout against loopback is the only honest answer to
//! "is the engine actually serving this session". A dead proxy must surface as
//! `alive: false`, never as an error - `aphrodite_stats` and the session-init
//! debug line both consume this value and must keep reporting.
//!
//! It also reports WHICH BUILD is answering. A port and a version cannot
//! distinguish the image that was hot-reloaded from the one it replaced, which is
//! the question this probe is consulted for after a rebuild; `Build` supplies the
//! stamp, and says in `identifies` whether that stamp names a revision or only a
//! version.

use crate::Health::{Build, Port};

/// Probe whether the cache and token proxies are listening, and stamp the build
/// that answered.
///
/// `proxy_health` is the name every caller outside this partition uses
/// (`ABI::ProxyHealth`, `Tool::Registry::{Stats,Rebuild}`, `Hook::Log`), so it
/// keeps the qualifier until those rows repoint; the leaf path is
/// `Health::Proxy::Fn`, and the bare `Fn` is the name this leaf wants.
pub(crate) fn proxy_health() -> serde_json::Value {
	use std::{net::TcpStream, time::Duration};
	let Timeout = Duration::from_millis(400);
	let Alive = |Address:String| {
		Address
			.parse()
			.ok()
			.and_then(|Socket| TcpStream::connect_timeout(&Socket, Timeout).ok())
			.is_some()
	};
	let (Cache, Token) = Port::Fn::Fn();
	serde_json::json!({
		"build": Build::Fn::Fn().json(),
		"token": {"port": Token, "alive": Alive(format!("127.0.0.1:{Token}"))},
		"cache": {"port": Cache, "alive": Alive(format!("127.0.0.1:{Cache}"))},
	})
}
