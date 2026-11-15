//! Binary entry: `--version`/`--help`/`setup` dispatch, the worker-thread count,
//! and the Tokio runtime the proxy runs on.
//!
//! These happen in this order for a reason. The early returns exist so
//! `--version`, `--help` and an install-only `setup` run never pay for a
//! runtime; a multi-thread pool built first would leave idle workers behind on a
//! command that has nothing left to serve.

use aphrodite::{
	Configuration::CLI::{Command::Enum::Enum as Command, Setup::Struct::Struct as SetupArgs, Struct::Struct as CLI},
	Setup,
};
use clap::Parser;

/// Run the `Aphrodite` binary to completion.
///
/// Owns argument dispatch and the runtime, then hands the proxy itself to
/// `crate::Boot::Run::run`. Returns `Ok(())` for `--version`, `--help` and a
/// completed `--no-launch` setup without ever starting a listener; a failed
/// setup exits the process directly with code 1 rather than returning, because
/// the caller has nothing left to do with the error.
pub fn Fn() -> anyhow::Result<()> {
	// Handle --version / --help early to avoid starting the runtime.
	let Args:Vec<String> = std::env::args().collect();
	if Args.iter().any(|a| a == "--version" || a == "-V") {
		println!(
			"aphrodite v{}",
			option_env!("APHRODITE_VERSION").unwrap_or(env!("CARGO_PKG_VERSION"))
		);
		return Ok(());
	}
	if Args.get(1).map(String::as_str) == Some("--help") || Args.get(1).map(String::as_str) == Some("-h") {
		// CLI::parse() prints help/errors and exits the process internally.
		CLI::parse();
		return Ok(());
	}

	// ── Handle `aphrodite setup` subcommand ──
	// Parse early to check for setup before building the tokio runtime.
	if Args.get(1).map(String::as_str) == Some("setup") {
		let CLI = CLI::parse();
		if let Some(Command::Setup { APIKey, APIURL, Model, CachePort, TokenPort, NoLaunch, Force }) = CLI.command {
			let SetupArgs = SetupArgs { APIKey, APIURL, Model, CachePort, TokenPort, NoLaunch, Force };
			match Setup::Run::Entry::Fn(&SetupArgs) {
				Ok(()) => {
					if SetupArgs.NoLaunch {
						// --no-launch: setup done, don't start proxy.
						return Ok(());
					}
					// Default: setup complete, drop through to start proxy
					println!("setup complete, starting proxy...");
				},
				Err(e) => {
					eprintln!("setup failed: {e}");
					std::process::exit(1);
				},
			}
		}
	}

	// Worker thread count - I/O-bound proxy needs more than CPU cores.
	// Default: 4× CPU or 32 minimum. Override: APHRODITE_WORKER_THREADS.
	// F15: warn (via eprintln, not tracing - the subscriber isn't initialized
	// this early, see `run()`'s matching comment) on a malformed value
	// instead of silently falling back, mirroring `ApplyPortOverride`.
	let WorkerThreads = match std::env::var("APHRODITE_WORKER_THREADS") {
		Ok(v) => {
			match v.parse::<usize>() {
				Ok(n) => n,
				Err(_) => {
					eprintln!("APHRODITE_WORKER_THREADS={v:?} is not a valid number; using the computed default");
					let Cpus = std::thread::available_parallelism().map(|n| n.get()).unwrap_or(8);
					(Cpus * 4).max(32)
				},
			}
		},
		Err(_) => {
			let Cpus = std::thread::available_parallelism().map(|n| n.get()).unwrap_or(8);
			(Cpus * 4).max(32)
		},
	};
	let Runtime = tokio::runtime::Builder::new_multi_thread()
		.worker_threads(WorkerThreads)
		.enable_all()
		.build()
		.expect("tokio runtime");
	Runtime.block_on(crate::Boot::Run::Run())
}
