//! What the command line is asking the binary to do.

use aphrodite::Configuration::CLI::Setup::Struct::Struct as SetupArgs;

/// The one thing this process was asked to do.
///
/// The dispatch used to be four independent `match` statements over
/// `Args.iter().any(...)` and `Args.get(1)`, each with a `false => {}` arm - so
/// the reader had to run all four in their head to learn that `--version` beats
/// `--help` beats `setup` beats a plain run, and the precedence lived in the
/// order of the statements rather than in a value.
///
/// As a value the precedence is the order the alternatives are tried in
/// `Request::Fn`, and each branch below handles exactly one request instead of
/// re-testing the argument list.
pub(crate) enum Enum {
	/// `--version` or `-V`: print the version and exit, without a runtime.
	Version,
	/// `--help` or `-h`: let clap print help and exit the process.
	Help,
	/// The `setup` subcommand, already parsed into its arguments.
	Setup(SetupArgs),
	/// No special request: start the proxies.
	Run,
}
