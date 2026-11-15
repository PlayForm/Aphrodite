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
	match Args.iter().any(|a| a == "--version" || a == "-V") {
		true => {
			println!(
				"aphrodite v{}",
				option_env!("APHRODITE_VERSION").unwrap_or(env!("CARGO_PKG_VERSION"))
			);
			return Ok(());
		},
		false => {},
	}
	match Args.get(1).map(String::as_str) == Some("--help") || Args.get(1).map(String::as_str) == Some("-h") {
		true => {
			// CLI::parse() prints help/errors and exits the process internally.
			CLI::parse();
			return Ok(());
		},
		false => {},
	}

	// ── Handle `aphrodite setup` subcommand ──
	// Parse early to check for setup before building the tokio runtime.
	match Args.get(1).map(String::as_str) == Some("setup") {
		true => {
			let CLI = CLI::parse();
			match CLI.command {
				Some(Command::Setup { APIKey, APIURL, Model, CachePort, TokenPort, NoLaunch, Force }) => {
					let SetupArgs = SetupArgs { APIKey, APIURL, Model, CachePort, TokenPort, NoLaunch, Force };
					match Setup::Run::Entry::Fn(&SetupArgs) {
						Ok(()) => {
							match SetupArgs.NoLaunch {
								true => {
									// --no-launch: setup done, don't start proxy.
									return Ok(());
								},
								false => {},
							}
							// Default: setup complete, drop through to start proxy
							println!("setup complete, starting proxy...");
						},
						Err(e) => {
							// The subscriber cannot exist on this path yet: `setup`
							// runs before the configuration is parsed, so this is
							// the pre-subscriber sink rather than a `tracing`
							// event that would be dropped.
							aphrodite::Logging::Stderr::Fn::Fn(&format!("setup failed: {e:#}"));
							std::process::exit(1);
						},
					}
				},
				Some(Command::Run) => {},
				None => {},
			}
		},
		false => {},
	}

	// Worker thread count - I/O-bound proxy needs more than CPU cores.
	// Default: 4× CPU or 32 minimum. Override: APHRODITE_WORKER_THREADS.
	// F15: warn on a malformed value instead of silently falling back,
	// mirroring `ApplyPortOverride`. The pre-subscriber sink, not `tracing`:
	// the worker pool is built before `Boot::Run` parses the configuration, so
	// no subscriber exists yet and a `tracing::warn!` would be dropped.
	let WorkerThreads = match std::env::var("APHRODITE_WORKER_THREADS") {
		Ok(v) => {
			match v.parse::<usize>() {
				Ok(n) => n,
				Err(_) => {
					aphrodite::Logging::Stderr::Fn::Fn(&format!(
						"APHRODITE_WORKER_THREADS={v:?} is not a valid number; using the computed default"
					));
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
