//! Reading the request off the argument list.

use super::{Enum::Enum, Setup};

/// Decide what this process was asked to do.
///
/// Precedence, highest first: `--version`, `--help`, `setup`, a plain run. The
/// chain is `or_else`, so an earlier request that matched means the later ones
/// are never evaluated - `CLI::parse` does not run on a `--version` invocation,
/// which is the whole reason `--version` is checked before anything else: a
/// package manager probing it must not pay for clap or for a runtime.
///
/// `--version`/`-V` is searched for anywhere in the argument list because that is
/// what the original did; `--help`/`-h` and `setup` are only recognised in the
/// first position, which is where clap itself looks for them.
pub(crate) fn Fn(Args:&[String]) -> Enum {
	let First = || Args.get(1).map(String::as_str);

	Args.iter()
		.any(|a| a == "--version" || a == "-V")
		.then_some(Enum::Version)
		.or_else(|| matches!(First(), Some("--help") | Some("-h")).then_some(Enum::Help))
		.or_else(|| Setup::Fn::Fn(First()))
		.unwrap_or(Enum::Run)
}
