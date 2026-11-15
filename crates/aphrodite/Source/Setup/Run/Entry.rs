//! The setup entry point.
//!
//! One function owns the whole install order, and now it owns it as a value: the
//! plan resolves the two homes, decides the two conditions and produces the ordered
//! step list without writing anything, then the executor performs that list and
//! returns what completed. The order is still the contract - resolve both homes,
//! verify prerequisites, create the directories, copy the binary, install the
//! dylibs, write configuration, write the loader, pin the version, register the
//! plugin - but it is now readable before it runs.

use crate::{
	Configuration::CLI::Setup::Struct::Struct as SetupArgs,
	Setup::{Execute, Plan, Verify::Hermes::Fn as VerifyHermes},
};

/// Run the setup/bootstrap process.
///
/// Resolves the Hermes home and the runtime home, then installs everything the
/// plugin needs: the binary and dylibs into `<runtime-home>/binaries/`, the
/// configuration template into `<runtime-home>/aphrodite.toml`, and the hooks-only
/// loader into `<hermes-home>/plugins/aphrodite/`. A catalog install - a real plugin
/// directory under the Hermes home, placed there by `download.sh`/`download.ps1` - is
/// report-only: setup prints what it would have done and touches nothing in it.
///
/// It is not idempotent in the "only what changed" sense and it is not a repair tool:
/// the binary, the dylibs, the manifest, the loader and the version pin are always
/// rewritten, only `aphrodite.toml` is preserved unless `--force`, and a run that
/// fails halfway leaves the artifacts it already wrote. Re-running is the repair.
///
/// It is also not safe to call twice concurrently, and it must resolve the same homes
/// the plugin will: under a non-default Hermes home (Docker, profile gateways) setup
/// writing to a second `~/.hermes/aphrodite` produces an install that looks complete
/// and never loads.
///
/// The prerequisite check runs between the plan and the execution on purpose: it is
/// read-only, and a missing `hermes` CLI must be reported before anything is written.
pub fn Fn(Args:&SetupArgs) -> anyhow::Result<()> {
	let Plan = Plan::Fn::Fn(Args)?;

	println!("aphrodite setup v{}", env!("CARGO_PKG_VERSION"));
	println!("   self-hash: {}", Plan.Context.OwnHash);

	VerifyHermes()?;

	let Report = Execute::Fn::Fn(&Plan, Args)?;

	println!("aphrodite installed -> {}", Plan.Context.AphroditeDirectory.display());
	println!(
		"  binaries, config, and state: {} (everything the plugin manages)",
		Plan.Context.AphroditeDirectory.display()
	);

	let _ = Plan.CatalogInstall.then(|| {
		println!(
			"  plugin dir (catalog install, report-only): {} - loader NOT written by setup; use \
			 download.sh/download.ps1",
			Plan.Context.PluginDirectory.display()
		);
	});

	let _ = (!Plan.CatalogInstall).then(|| {
		println!("  hooks registered (loader only): {}", Plan.Context.PluginDirectory.display());
	});

	let _ = Report;

	Ok(())
}
