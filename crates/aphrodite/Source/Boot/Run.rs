//! The startup orchestration: configuration, listener binding, and the shutdown
//! drain, in the one order that keeps a half-started proxy from looking healthy.
//!
//! Every step here is a call into a leaf; this file holds no rule of its own. If
//! a line in `Run` grows logic, it belongs in `Configuration/` or `Listen/`
//! instead - the value of this file is that the whole startup sequence reads in
//! one screen.

use crate::Boot::{Banner::Announce, Configuration, Listen};

/// Start every configured proxy and block until the process is asked to stop.
///
/// Returns `Err` only for a failure that must abort startup: a config that will
/// not parse, a listener that will not bind, or an `AppState` that cannot be
/// built. A listener that fails *after* binding is logged by its own task and
/// does not fail this function - which is exactly why a bind failure is fatal
/// here but a serve failure is not.
pub async fn Run() -> anyhow::Result<()> {
	let (ConfigurationPath, UseMultiConfiguration) = Configuration::Path::Resolve();
	let (MultiConfiguration, CLIFallback, LogCompact) =
		Configuration::Load::Load(&ConfigurationPath, UseMultiConfiguration)?;
	Configuration::Tracing::Subscribe(LogCompact)?;
	let Compression = Configuration::Compression::Compression(MultiConfiguration.as_ref());
	let Proxies = Configuration::Proxies::Assemble(MultiConfiguration, CLIFallback)?;
	Announce(Proxies.len());

	// ── Shared shutdown watch channel ────────────────────────────────
	// Initial value false; main() sets true on first Ctrl+C/SIGTERM,
	// each run_single() task waits on the receiver for graceful_shutdown.
	let (ShutdownTx, ShutdownRx) = tokio::sync::watch::channel(false);

	let Bound = Listen::Bind::Bind(Proxies, Compression.as_ref()).await?;
	Listen::Watcher::watch(&ConfigurationPath, &Bound);
	let Handles = Listen::Spawn::Spawn(Bound, ShutdownRx);
	Listen::Drain::Drain(ShutdownTx, Handles).await;

	Ok(())
}
