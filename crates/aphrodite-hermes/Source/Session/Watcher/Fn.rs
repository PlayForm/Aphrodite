//! Spawn the config auto-reload watcher.
//!
//! The watcher is opt-in (`[compression] auto_reload = true`, default off) and
//! started once per process from the config template. It reaches EVERY
//! registered session, so a config change applies to the parent and all
//! subagent sessions alike - each under its own bounded lock, wedged ones
//! counted, because a reload must never block the gateway.
//!
//! This leaf is the composition: it spawns the thread, waits for it to report
//! whether the notifier came up, and parks the resulting handle. `Run/` owns
//! the loop, `Wait/` the wake, `Apply/` the per-session application, `Continue/`
//! the keep-watching policy, and `Source/` the notifier itself - all of which
//! are compiled in the test build because none of them needs a thread.

use std::{
	sync::{
		Arc,
		atomic::AtomicBool,
		mpsc::{channel, sync_channel},
	},
	time::Duration,
};

use crate::Session::Watcher::{Handle, Ready, Run, Start, State};

/// How long the spawner waits for the thread's readiness report.
///
/// Bounded because this runs inside the config template's first initialisation,
/// which a hook is waiting on: an unbounded wait here would turn "the notifier
/// could not start" into "the first hook never returned". Two seconds is far
/// above the cost of building a notifier and watching two directories, and far
/// below Hermes' hook timeout.
const READY_TIMEOUT:Duration = Duration::from_secs(2);

/// Spawn the opt-in config auto-reload watcher and report whether it is running.
///
/// Mirrors the engine binary's watcher (`Main.rs`): watch the config search dirs
/// (cwd, then the user runtime home - the same precedence as `Config::load()`)
/// non-recursively, debounce 500ms on a modify of `aphrodite.toml`, then reload.
///
/// Failure policy (fail-open, `aphrodite-boundaries`): a broken file keeps
/// the previous values (like the engine watcher) and records the failure on
/// `config_error` so `aphrodite_stats` shows "defaults in effect (parse
/// failed)"; the loader's visible warn (stderr fallback when no tracing
/// subscriber exists) fires on its own. Setting `auto_reload = false` in the
/// file stops the watcher on the next successful reload.
///
/// A watcher that cannot start is reported, not swallowed: the caller asked for
/// auto-reload, and "it is silently off" is the one answer it cannot act on.
pub(crate) fn spawn_config_watcher() -> Start::Enum::Enum {
	let Stop = Arc::new(AtomicBool::new(false));
	let (Wake, Receiver) = channel::<()>();
	let (ReadySender, ReadyReceiver) = sync_channel::<Ready::Enum::Enum>(1);

	let Spawned = std::thread::Builder::new().name("aphrodite-config-watcher".into()).spawn({
		let Stop = Arc::clone(&Stop);
		let Wake = Wake.clone();

		move || Run::Fn::Fn(Receiver, Stop, Wake, ReadySender)
	});

	let Thread = match Spawned {
		Ok(Thread) => Thread,
		Err(e) => {
			crate::Debug::Warn::Fn::Fn(&format!("config auto-reload watcher failed to start: {e}"));

			return Start::Enum::Enum::NotRunning;
		},
	};

	let Running = Handle::Struct::Struct { Stop, Wake, Joined:Thread };

	match ReadyReceiver.recv_timeout(READY_TIMEOUT) {
		Ok(Ready::Enum::Enum::Watching) => {
			let Previous = {
				let mut Slot = State::RUNNING
					.get_or_init(|| std::sync::Mutex::new(None))
					.lock()
					.unwrap_or_else(std::sync::PoisonError::into_inner);

				Slot.replace(Running)
			};

			// A second spawn replaces the first: two threads on one config file
			// would apply every change twice, and the replaced thread would be
			// unreachable - running this image's code with no way to join it.
			if let Some(Old) = Previous {
				Old.stop();
			}

			Start::Enum::Enum::Running
		},
		// The thread reported that it could not build the notifier and has
		// already returned; joining is what proves it is gone.
		Ok(Ready::Enum::Enum::Failed(Reason)) => {
			crate::Debug::Warn::Fn::Fn(&Reason);
			Running.stop();

			Start::Enum::Enum::NotRunning
		},
		// No report at all: the thread is stuck before readiness, which is a
		// state the loop cannot be in (readiness is sent before the first
		// `recv`), so stop it rather than leave an unreachable thread behind.
		Err(e) => {
			crate::Debug::Warn::Fn::Fn(&format!(
				"config auto-reload watcher did not report readiness within {}ms ({e}); stopping it",
				READY_TIMEOUT.as_millis()
			));
			Running.stop();

			Start::Enum::Enum::NotRunning
		},
	}
}
