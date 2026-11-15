//! Binary entry: what the command line asked for, the worker-thread count, and
//! the Tokio runtime the proxy runs on.
//!
//! These happen in this order for a reason. The early returns exist so
//! `--version`, `--help` and an install-only `setup` run never pay for a
//! runtime; a multi-thread pool built first would leave idle workers behind on a
//! command that has nothing left to serve.
//!
//! The argument list is read once, into a `Request` value, and the rest of this
//! function is one match over it. Nothing here re-tests the arguments, so the
//! precedence between the four requests lives in `Request::Fn` and nowhere else.

use aphrodite::{Configuration::CLI::Struct::Struct as CLI, Setup};
use clap::Parser;

use crate::Boot::{Request, Worker};

/// Run the `Aphrodite` binary to completion.
///
/// Owns argument dispatch and the runtime, then hands the proxy itself to
/// `crate::Boot::Run::Run`. Returns `Ok(())` for `--version`, `--help` and a
/// completed `--no-launch` setup without ever starting a listener; a failed
/// setup exits the process directly with code 1 rather than returning, because
/// the caller has nothing left to do with the error.
pub fn Fn() -> anyhow::Result<()> {
	let Args:Vec<String> = std::env::args().collect();

	match Request::Fn::Fn(&Args) {
		Request::Enum::Enum::Version => {
			println!(
				"aphrodite v{}",
				option_env!("APHRODITE_VERSION").unwrap_or(env!("CARGO_PKG_VERSION"))
			);

			return Ok(());
		},
		Request::Enum::Enum::Help => {
			// CLI::parse() prints help/errors and exits the process internally.
			CLI::parse();

			return Ok(());
		},
		Request::Enum::Enum::Setup(SetupArgs) => {
			match Setup::Run::Entry::Fn(&SetupArgs) {
				Ok(()) => {
					// --no-launch: setup done, don't start the proxy.
					if SetupArgs.NoLaunch {
						return Ok(());
					}

					println!("setup complete, starting proxy...");
				},
				Err(e) => {
					// The subscriber cannot exist on this path yet: `setup` runs
					// before the configuration is parsed, so this is the
					// pre-subscriber sink rather than a `tracing` event that
					// would be dropped.
					aphrodite::Logging::Stderr::Fn::Fn(&format!("setup failed: {e:#}"));

					std::process::exit(1);
				},
			}
		},
		Request::Enum::Enum::Run => {},
	}

	let Runtime = tokio::runtime::Builder::new_multi_thread()
		.worker_threads(Worker::Count::Fn::Fn())
		.enable_all()
		.build()
		.expect("tokio runtime");

	Runtime.block_on(crate::Boot::Run::Run())
}
