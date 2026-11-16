//! The startup orchestration: configuration, listener binding, and the shutdown
//! drain, in the one order that keeps a half-started proxy from looking healthy.
//!
//! Every step here is a call into a leaf; this file holds no rule of its own. If
//! a line in `Run` grows logic, it belongs in `Configuration/` or `Listen/`
//! instead - the value of this file is that the whole startup sequence reads in
//! one screen.

use crate::Boot::{
	Banner::Announce,
	Configuration::{self, Source},
	Listen,
};

/// Start every configured proxy and block until the process is asked to stop.
///
/// Returns `Err` only for a failure that must abort startup: a config that will
/// not parse, a listener that will not bind, or an `AppState` that cannot be
/// built. A listener that fails *after* binding is logged by its own task and
/// does not fail this function - which is exactly why a bind failure is fatal
/// here but a serve failure is not.
///
/// The configuration is loaded once and carried as one value. Nothing below the
/// load asks "is there a file?" again: `Source` is either the document or the
/// command line, and each consumer matches the arm it cares about.
pub async fn Run() -> anyhow::Result<()> {
	let Resolved = Configuration::Path::Fn::Fn();
	let Loaded = Configuration::Load::Fn::Fn(&Resolved)?;

	// The configured level: `--dev` is the one knob that names a verbosity
	// ("verbose request/response logging"), so it is the configured `debug`.
	// Everything else defers to `APHRODITE_LOG`, then `RUST_LOG`, then `info` -
	// see `Logging::Level`. It is a command-line knob, so it has no meaning when
	// the document is the configuration.
	let ConfiguredLevel = match &Loaded.Source {
		Source::Enum::Enum::Standalone(CLI) => CLI.Dev.then_some(tracing::Level::DEBUG),
		Source::Enum::Enum::Multi(_) => None,
	};

	aphrodite::Logging::Fn::Fn(Loaded.LogCompact, ConfiguredLevel)?;

	let Compression = Configuration::Compression::Fn::Fn(&Loaded.Source);
	let Proxies = Configuration::Proxies::Fn::Fn(Loaded.Source)?;

	Announce(Proxies.len());

	// ── Shared shutdown watch channel ────────────────────────────────
	// Initial value false; main() sets true on first Ctrl+C/SIGTERM,
	// each serving task waits on the receiver for graceful_shutdown.
	let (ShutdownTx, ShutdownRx) = tokio::sync::watch::channel(false);

	let Bound = Listen::Bind::All::Fn::Fn(Proxies, Compression.as_ref()).await?;

	// The watch is started before anything serves and stopped after the drain: its
	// readiness is a value (`Watching`/`Inactive`), and stopping it is a step rather
	// than a task left to process exit.
	let Watcher = Listen::Watcher::Fn::Fn(&Resolved.Path, &Bound);

	let Handles = Listen::Spawn::Spawn(Bound, ShutdownRx);

	Listen::Drain::Drain(ShutdownTx, Handles).await;

	Listen::Watcher::Stop::Fn::Fn(Watcher);

	Ok(())
}
