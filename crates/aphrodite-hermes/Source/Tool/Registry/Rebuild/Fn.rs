//! `aphrodite_rebuild` - the operational state report.

use Tool::Dispatch::Fn::ToolHandler;
use Health::Proxy::Fn::proxy_health;

/// The `aphrodite_rebuild` handler.
///
/// The dylib cannot safely rebuild itself mid-session, so this surfaces the state
/// the operator needs and lets the standalone proxy / dev loop do the rebuild. The
/// `hint` string is the contract: it names the exact command to run.
pub(crate) fn Fn() -> (&'static str, ToolHandler) {
	("aphrodite_rebuild", |_args| {
		serde_json::json!({
			"status": "ok",
			"version": env!("CARGO_PKG_VERSION"),
			"proxies": proxy_health(),
			"hint": "rebuild via `cargo build --release -p aphrodite`; restart the session to pick up the new dylib",
		})
	})
}
