//! The production wiring: the only part of the watcher that needs a real
//! filesystem thread.

use std::sync::Arc;

use aphrodite::Proxy::State::Struct::Struct as AppState;

use super::{Loop, Source};
use crate::Boot::State::Listener::Bound;

/// Watch `ConfigurationPath`'s directory and apply threshold changes to every
/// listener.
///
/// Returns immediately - the watcher runs as its own task for the life of the
/// process, and a watcher that cannot be created or started warns from inside
/// that task rather than failing startup.
///
/// The path is made absolute against the current directory before the watch
/// starts, because `notify` watches a *directory* and a relative path would
/// resolve against whatever the process later does with its cwd. The path is
/// still reported as the operator wrote it where that is possible; only the
/// watch directory needs the absolute form.
///
/// Everything below this function is free of `notify`, which is what lets
/// `Loop` be driven from a channel a test owns.
pub fn Fn(ConfigurationPath:&str, Bound:&[Bound]) {
	let RawPath = std::path::PathBuf::from(ConfigurationPath);
	// `std::path::absolute` joins against the current directory without touching
	// the filesystem; when the cwd is unavailable the operator's own value is the
	// honest thing to watch.
	let WatchPath = std::path::absolute(&RawPath).unwrap_or(RawPath);
	let WatchPathStr = WatchPath.to_string_lossy().to_string();
	let States:Vec<Arc<AppState>> = Bound.iter().map(|b| b.State.clone()).collect();

	tokio::spawn(async move {
		let Some(Source) = Source::Fn::Fn(&WatchPath) else {
			return;
		};

		tracing::info!(path = %WatchPathStr, "config file watcher active");

		Loop::Fn::Fn(Source, States, WatchPathStr).await;
	});
}
