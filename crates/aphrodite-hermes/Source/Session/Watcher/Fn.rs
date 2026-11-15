//! Spawn the config auto-reload watcher.
//!
//! The watcher is opt-in (`[compression] auto_reload = true`, default off) and
//! started once per process from the config template. It reaches EVERY
//! registered session, so a config change applies to the parent and all
//! subagent sessions alike - each under its own bounded lock, wedged ones
//! skipped, because a reload must never block the gateway.
//!
//! This leaf is the composition and nothing else: `Debounce/` owns the settle
//! window, `Apply/` owns the per-session application, and both are compiled in
//! the test build because neither needs a thread.

use crate::Session::Watcher::{Apply, Debounce};

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
			let (Sender, Receiver) = std::sync::mpsc::channel::<()>();
			let mut watcher = match notify::recommended_watcher(move |Result:Result<notify::Event, notify::Error>| {
				let Wanted = Result.is_ok_and(|Event| {
					matches!(Event.kind, notify::EventKind::Modify(_))
						&& Event.paths.iter().any(|Path| Path.to_string_lossy().contains("aphrodite.toml"))
				});

				let _ = Wanted.then(|| Sender.send(()));
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

			// One blocking tick per burst, then one apply. `Debounce` returns
			// false when the channel closed and `Apply` returns false when no
			// session still wants a reload (`auto_reload` turned off in the
			// file), so either one ends the watch.
			let mut Watching = true;

			while Watching && Debounce::Fn::Fn(&Receiver) {
				Watching = Apply::Fn::Fn(&aphrodite::ConfigurationLoader::Configuration::Struct::Struct::Load());
			}
		});
}
