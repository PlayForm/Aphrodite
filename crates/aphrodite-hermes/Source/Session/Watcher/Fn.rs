//! Spawn the config auto-reload watcher.
//!
//! The watcher is opt-in (`[compression] auto_reload = true`, default off) and
//! started once per process from the config template. It reaches EVERY
//! registered session, so a config change applies to the parent and all
//! subagent sessions alike - each under its own bounded lock, wedged ones
//! skipped, because a reload must never block the gateway.

use crate::{Configuration::Reload::Fn::apply_config_reload, Session::Broadcast::Fn::for_each_session};

/// Spawn the opt-in config auto-reload watcher. Mirrors the engine binary's
/// watcher (`Main.rs`): watch the config search dirs (cwd, then the user
/// runtime home - the same precedence as `Config::load()`) non-recursively,
/// debounce 500ms on a modify of `aphrodite.toml`, then reload.
///
/// A reload reaches EVERY registered session (each under its own bounded
/// lock, wedged ones skipped), so a config change applies to the parent and
/// all subagent sessions alike.
///
/// Failure policy (fail-open, `aphrodite-boundaries`): a broken file keeps
/// the previous values (like the engine watcher) and records the failure on
/// `config_error` so `aphrodite_stats` shows "defaults in effect (parse
/// failed)"; the loader's visible warn (stderr fallback when no tracing
/// subscriber exists) fires on its own. Setting `auto_reload = false` in the
/// file stops the watcher on the next successful reload.
pub(crate) fn spawn_config_watcher() {
	use notify::Watcher;
	let _ = std::thread::Builder::new()
		.name("aphrodite-config-watcher".into())
		.spawn(move || {
			let mut watch_dirs = vec![std::path::PathBuf::from(".")];
			// The runtime home is one shared decision (Home::runtime_home:
			// $APHRODITE_HOME -> $HERMES_HOME -> $HOME -> platform default), so
			// the watcher follows the same home the plugin shim resolves - under
			// a non-default Hermes home it must watch <hermes-home>/aphrodite,
			// never $HOME/.hermes/aphrodite (issue 40).
			watch_dirs.push(aphrodite::Home::Runtime::Fn::Fn());
			let (tx, rx) = std::sync::mpsc::channel::<()>();
			let mut watcher = match notify::recommended_watcher(move |res:Result<notify::Event, notify::Error>| {
				match res {
					Ok(ev)
						if matches!(ev.kind, notify::EventKind::Modify(_))
							&& ev.paths.iter().any(|p| p.to_string_lossy().contains("aphrodite.toml")) =>
					{
						let _ = tx.send(());
					},
					_ => {},
				}
			}) {
				Ok(w) => w,
				Err(e) => {
					crate::Debug::Warn::Fn::Fn(&format!("config auto-reload watcher failed to start: {e}"));

					return;
				},
			};

			watch_dirs.iter().for_each(|dir| {
				match watcher.watch(dir, notify::RecursiveMode::NonRecursive) {
					Ok(()) => {},
					Err(e) => {
						crate::Debug::Warn::Fn::Fn(&format!("config auto-reload: cannot watch {}: {e}", dir.display()))
					},
				}
			});

			// One blocking receive per event, until the channel closes: the
			// iterator IS the "wait for the next change" step, and the sweep
			// stops early (`Err`) when `auto_reload` turned off in the file.
			let _ = rx.iter().try_for_each(|()| {
				// Debounce: let the editor's write settle (same as the engine
				// watcher's 500ms), and drain events accumulated meanwhile.
				std::thread::sleep(std::time::Duration::from_millis(500));
				rx.try_iter().for_each(drop);

				let cfg = aphrodite::ConfigurationLoader::Configuration::Struct::Struct::Load();
				let keep_watching = match cfg.ParseFailure.clone() {
					// Broken file: keep the previous values, surface the
					// failure on EVERY session (the loader's warn already
					// fired); wedged sessions are skipped.
					Some(failure) => {
						for_each_session(|s| s.ConfigurationError = Some(failure.clone()));

						true
					},
					None => for_each_session(|s| apply_config_reload(&cfg, s)),
				};

				// `auto_reload` turned off in the file - stop watching
				// (takes effect on the next successful reload).
				match keep_watching {
					true => Ok(()),
					false => Err(()),
				}
			});
		});
}
