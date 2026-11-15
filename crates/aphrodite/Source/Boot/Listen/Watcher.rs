//! The config-file hot-reload watcher.
//!
//! Holds a clone of every listener's `Arc<AppState>` so a file change applies to
//! all of them (report 07 F4/T15 - this previously only re-parsed and logged,
//! because `AppState` did not exist yet at this point in startup and there was
//! nothing live for it to write into). Only the compression thresholds are
//! hot-applied; everything else a config file describes needs a restart.

use std::sync::{Arc, atomic::Ordering};

use aphrodite::{
	Configuration::Proxy::Multi::Struct::Struct as MultiConfiguration,
	Proxy,
	Proxy::State::Struct::Struct as AppState,
};

use Boot::State::Listener::Bound;

/// Watch `ConfigurationPath`'s directory and apply threshold changes to every listener.
///
/// Returns immediately - the watcher runs as its own task for the life of the
/// process. A watcher that cannot be created or started logs a warning and
/// returns rather than failing startup: hot reload is a convenience, and a
/// proxy that refuses to start because `notify` failed is a worse outcome than
/// one that needs a restart to pick up a new threshold.
pub fn watch(ConfigurationPath:&str, Bound:&[Bound]) {
	// ── Spawn config file watcher for hot-reload ──────────────────
	// Holds a clone of every listener's `Arc<AppState>` so a file change
	// applies to all of them (report 07 F4/T15 - previously this only
	// re-parsed and logged; `AppState` didn't exist yet at this point in
	// startup, so there was nothing live for it to write into).
	let WatchPath = {
		let p = std::path::PathBuf::from(ConfigurationPath);
		if p.is_relative() {
			std::env::current_dir().unwrap_or_default().join(&p)
		} else {
			p
		}
	};
	let WatchPathStr = WatchPath.to_string_lossy().to_string();
	let StatesForWatcher:Vec<Arc<AppState>> = Bound.iter().map(|b| b.State.clone()).collect();
	tokio::spawn(async move {
		use notify::{Event, EventKind, RecursiveMode, Watcher};
		let (Tx, mut Rx) = tokio::sync::mpsc::channel(16);
		let mut Watcher = match notify::recommended_watcher(move |Res:Result<Event, notify::Error>| {
			if let Ok(Event) = Res {
				let IsModify = matches!(Event.kind, EventKind::Modify(_));
				if IsModify && Event.paths.iter().any(|p| p.to_string_lossy().contains("aphrodite.toml")) {
					let _ = Tx.try_send(());
				}
			}
		}) {
			Ok(w) => w,
			Err(e) => {
				tracing::warn!("failed to create config watcher: {e}");
				return;
			},
		};
		let WatchDirectory = std::path::Path::new(&WatchPath).parent().unwrap_or(std::path::Path::new("."));
		if let Err(e) = Watcher.watch(WatchDirectory, RecursiveMode::NonRecursive) {
			tracing::warn!("failed to start config watcher: {e}");
			return;
		}
		tracing::info!(path = %WatchPathStr, "config file watcher active");
		loop {
			if Rx.recv().await.is_some() {
				// Debounce: wait 500ms for writes to settle
				tokio::time::sleep(std::time::Duration::from_millis(500)).await;
				// Drain any additional events accumulated during debounce
				while Rx.try_recv().is_ok() {}
				match MultiConfiguration::Load(&WatchPathStr) {
					Ok(Configuration) => {
						let Thresholds = Proxy::Threshold::Resolve::Fn::Fn(Configuration.Compression.as_ref());
						for State in &StatesForWatcher {
							State.CacheCompressThreshold.store(Thresholds.Cache, Ordering::Relaxed);
							State.TokenCompressThreshold.store(Thresholds.Token, Ordering::Relaxed);
							State.InlineCCRThreshold.store(Thresholds.inline, Ordering::Relaxed);
							State
								.CodeMultiplierX100
								.store((Thresholds.CodeMultiplier * 100.0) as u64, Ordering::Relaxed);
						}
						tracing::info!(
							path = %WatchPathStr,
							CacheThreshold = Thresholds.Cache,
							TokenThreshold = Thresholds.Token,
							InlineThreshold = Thresholds.inline,
							CodeMultiplier = Thresholds.CodeMultiplier,
							Listeners = StatesForWatcher.len(),
							"config reloaded - compression thresholds applied to all live listeners"
						);
					},
					Err(e) => {
						tracing::warn!(error = %e, "failed to reload config on file change");
					},
				}
			}
		}
	});
}
