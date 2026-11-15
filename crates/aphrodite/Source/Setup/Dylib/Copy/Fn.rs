//! The shared-library install pass.

use std::path::{Path, PathBuf};

use crate::Setup::Run::Context::Struct as SetupContext;
use super::{super::Download, Install, Locate, Names, Source, Unavailable};

/// Install both dylibs: from disk when they are there, from the release when they
/// are not.
///
/// Release assets carry the target triple and the installed file must not:
/// `libaphrodite_hermes-aarch64-apple-darwin.dylib` is fetched and saved as
/// `libaphrodite_hermes.dylib`, which is the name the loader opens. A local hit
/// takes the platform's copy treatment, which on macOS is what keeps the copied
/// dylib loadable instead of quarantine-killed.
///
/// It is not transactional: artifacts copied before a failure stay in place, so a
/// failed run can leave a partial `binaries/` directory and a re-run is the
/// repair. It also never removes a dylib it did not install - an optional core
/// library that cannot be found is skipped with a warning, because setup
/// completing without it is the correct outcome and aborting there would make the
/// documented `cargo install` path unusable.
///
/// The `Requirement` variant decides that: `Required` turns an unreachable dylib
/// into an error naming the build command and the release page, `Optional` warns
/// and moves to the next one.
pub(crate) fn Fn(Context:&SetupContext) -> anyhow::Result<()> {
	let ExeDirectory = Context.OwnPath.parent().unwrap_or(Path::new("."));
	let SearchPaths:Vec<PathBuf> = vec![
		ExeDirectory.to_path_buf(),
		ExeDirectory.join("deps"),
		PathBuf::from("/usr/local/lib"),
		PathBuf::from("/opt/homebrew/lib"),
	];

	let mut Copied = 0u32;

	Names::Const::Const
		.iter()
		.try_for_each(|&(Name, Requirement)| -> anyhow::Result<()> {
			let Dest = Context.BinariesDirectory.join(Name);

			match Locate::Fn::Fn(Name, &SearchPaths, Locate::Target::Fn::Fn(ExeDirectory, Name)) {
				Source::Enum::Enum::Search(Src) | Source::Enum::Enum::Build(Src) => {
					println!("copying dylib {} -> {}", Name, Dest.display());

					Install::Fn::Fn(&Src, &Dest, Name)?;

					Copied += 1;
				},
				Source::Enum::Enum::Downloaded => {
					match Download::Fn::Fn(Name, &Dest) {
						Ok(()) => Copied += 1,
						Err(e) => Unavailable::Fn::Fn(Requirement, Name, e)?,
					}
				},
			}

			Ok(())
		})?;

	println!("copied {Copied} dylib(s)");

	Ok(())
}
