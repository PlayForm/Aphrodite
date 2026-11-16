//! Binary entry: what the command line asked for, the worker-thread count, and
//! the Tokio runtime the proxy runs on.
//!
//! These happen in this order for a reason. A request with nothing left to serve
//! never pays for a runtime; a multi-thread pool built first would leave idle
//! workers behind on `--version`, `--help` or an install-only `setup`.
//!
//! The argument list is read once, into a `Request` value, and the rest of this
//! function is one match over it. Nothing here re-tests the arguments, so the
//! precedence between the four requests lives in `Request::Fn` and nowhere else.
//! The same holds one level down: whether a completed `setup` starts the proxy is
//! `--no-launch` read once, as a value, not a branch between two statements.

use anyhow::Context;
use aphrodite::{
	Configuration::CLI::{Setup::Struct::Struct as SetupArgs, Struct::Struct as CLI},
	Setup,
};
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

			Ok(())
		},
		Request::Enum::Enum::Help => {
			// CLI::parse() prints help/errors and exits the process internally.
			CLI::parse();

			Ok(())
		},
		Request::Enum::Enum::Setup(SetupArgs) => AfterSetup(&SetupArgs),
		Request::Enum::Enum::Run => Serve(),
	}
}

/// A completed `setup` run: `--no-launch` stops here, anything else starts the proxy.
fn AfterSetup(Args:&SetupArgs) -> anyhow::Result<()> {
	match Setup::Run::Entry::Fn(Args) {
		Ok(()) => Args.NoLaunch.then_some(()).map_or_else(Launched, |()| Ok(())),
		Err(e) => {
			// The subscriber cannot exist on this path yet: `setup` runs
			// before the configuration is parsed, so this is the
			// pre-subscriber sink rather than a `tracing` event that
			// would be dropped.
			aphrodite::Logging::Stderr::Fn::Fn(&format!("setup failed: {e:#}"));

			std::process::exit(1);
		},
	}
}

/// Hand over to the proxy, saying so once.
fn Launched() -> anyhow::Result<()> {
	println!("setup complete, starting proxy...");

	Serve()
}

/// Build the runtime and run the proxy on it.
fn Serve() -> anyhow::Result<()> {
	// A runtime that cannot be built is a startup failure, not an invariant: the
	// caller already has a `Result`, so it is reported through it rather than
	// asserted. An `expect` here would turn "the host refused a thread pool" into a
	// panic with no context on a path that has a perfectly good error channel.
	let Runtime = tokio::runtime::Builder::new_multi_thread()
		.worker_threads(Worker::Count::Fn::Fn())
		.enable_all()
		.build()
		.context("build the tokio runtime")?;

	Runtime.block_on(crate::Boot::Run::Run())
}
