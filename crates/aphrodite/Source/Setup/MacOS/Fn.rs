//! Copy `Src` to `Dest` with the full macOS treatment, then harden permissions.
//!
//! `DylibIDName` is `Some(name)` for a dylib copy - the install name is rewritten
//! to `@rpath/<name>` and the artifact is ad-hoc re-signed - and `None` for a
//! plain binary copy, which has no dylib ID to rewrite and no
//! signature-invalidating relink.
//!
//! It is not best-effort. A non-zero exit from `ditto` counts as failure and
//! takes the fallback, and every tool that is missing or fails warns with the
//! consequence named, because a swallowed `let _ =` here is what made the SIGKILL
//! bug look intermittent on source builds. Getting this wrong leaves an artifact
//! at the install path that macOS refuses to load, and the only symptom is a
//! plugin that never starts.
//!
//! Each tool's outcome is folded into the one fact the code branches on - did it
//! succeed - with `is_ok_and`, so a spawn failure and a non-zero exit are the same
//! answer and neither needs its own arm.
//!
//! Every path is passed as an `OsStr` argument. The old calls went through
//! `Dest.to_str().unwrap_or("")`, which on a path that is not valid UTF-8 handed
//! the tools an empty string: `ditto` copied nothing, `xattr` cleaned nothing, and
//! `install_name_tool`/`codesign` silently "succeeded" against no file.

use std::{fs, path::Path};

use crate::{
	Logging::Stderr::Fn::Fn as StderrLine,
	Setup::{
		MacOS::{Copy, Dylib, Outcome},
		Write::Permissions::Fn as SecurePerms,
	},
};

/// Copy `Src` to `Dest` with the full macOS treatment, then harden permissions.
pub fn Fn(Src:&Path, Dest:&Path, DylibIDName:Option<&str>, Mode:u32) -> anyhow::Result<()> {
	let _ = fs::remove_file(Dest);

	let Copied = Copy::Fn::Fn(Src, Dest)?;
	let Treated = DylibIDName.map_or_else(Vec::new, |Name| Dylib::Fn::Fn(Dest, Name));

	std::iter::once(Copied)
		.chain(Treated)
		.filter_map(Outcome::Warning::Fn::Fn)
		.for_each(|Warning| StderrLine(&Warning));

	SecurePerms(Dest, Mode)?;

	Ok(())
}
