//! The copy `ditto` could not do.

use std::{fs, path::Path};

use anyhow::Context;

use crate::Setup::MacOS::Outcome;

/// Copy with `fs::copy`, then strip extended attributes with `xattr -c`.
///
/// `fs::copy` preserves the quarantine attribute, so the `xattr` call is what
/// keeps Gatekeeper from killing the artifact; when the tool is missing the copy
/// still happened and only the consequence needs saying.
pub fn Fn(Src:&Path, Dest:&Path) -> anyhow::Result<Outcome::Enum::Enum> {
	fs::copy(Src, Dest).with_context(|| format!("copy {} to {}", Src.display(), Dest.display()))?;
	let Cleaned = std::process::Command::new("xattr")
		.arg("-c")
		.arg(Dest)
		.output()
		.is_ok_and(|o| o.status.success());
	Ok(Cleaned.then_some(Outcome::Enum::Enum::Done).unwrap_or_else(|| {
		Outcome::Enum::Enum::Warned(format!(
			"warning: xattr -c failed or unavailable for {} - Gatekeeper may still kill this artifact",
			Dest.display()
		))
	}))
}
