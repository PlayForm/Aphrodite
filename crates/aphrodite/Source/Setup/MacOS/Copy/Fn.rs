//! The copy step: `ditto` when it works, the plain copy plus `xattr -c` when not.

use std::path::Path;

use crate::Setup::MacOS::{Fallback, Outcome};

/// Copy `Src` to `Dest`, preferring `ditto`.
///
/// `ditto` is the only copy that does not carry the quarantine attribute across,
/// so a non-zero exit - not just a spawn failure - takes the fallback: both are
/// "the tool did not do the job" and neither needs its own arm.
pub fn Fn(Src:&Path, Dest:&Path) -> anyhow::Result<Outcome::Enum::Enum> {
	let Copied = std::process::Command::new("ditto")
		.arg(Src)
		.arg(Dest)
		.status()
		.is_ok_and(|s| s.success());
	Copied
		.then_some(Outcome::Enum::Enum::Done)
		.map_or_else(|| Fallback::Fn::Fn(Src, Dest), Ok)
}
