//! The dylib treatment: rewrite the install name, then re-sign.

use std::path::Path;

use crate::Setup::MacOS::Outcome;

/// Rewrite the install name to `@rpath/<Name>` and ad-hoc re-sign.
///
/// Both steps exist because `install_name_tool` invalidates the dylib's embedded
/// signature on arm64, so a rewritten dylib that is not re-signed is one macOS
/// refuses to load. Two outcomes, in the order the tools ran.
pub fn Fn(Dest:&Path, Name:&str) -> Vec<Outcome::Enum::Enum> {
	let Rpath = format!("@rpath/{Name}");
	let Rewritten = std::process::Command::new("install_name_tool")
		.args(["-id", &Rpath])
		.arg(Dest)
		.output()
		.is_ok_and(|o| o.status.success());
	let Signed = std::process::Command::new("codesign")
		.args(["-f", "-s", "-"])
		.arg(Dest)
		.output()
		.is_ok_and(|o| o.status.success());
	vec![
		Rewritten.then_some(Outcome::Enum::Enum::Done).unwrap_or_else(|| {
			Outcome::Enum::Enum::Warned(format!(
				"warning: install_name_tool failed or unavailable for {} - Gatekeeper may kill Hermes when loading \
				 this dylib; install Xcode Command Line Tools and re-run setup",
				Dest.display()
			))
		}),
		Signed.then_some(Outcome::Enum::Enum::Done).unwrap_or_else(|| {
			Outcome::Enum::Enum::Warned(format!(
				"warning: codesign failed or unavailable for {} - Gatekeeper may still kill this dylib",
				Dest.display()
			))
		}),
	]
}
