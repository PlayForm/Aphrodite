//! Where a dylib is found, in fallback order.

use std::path::PathBuf;

use super::super::Source;

/// Find `Name` in the search paths, then in the build output, then nowhere.
///
/// The order is the policy and it is lazy: the build-output check runs only when
/// every search path missed, and `Downloaded` is the answer only when neither
/// produced a file. A `cargo install aphrodite && aphrodite setup` user has no
/// source checkout, so they always land on `Downloaded`; a dev build usually
/// finds the dylib in `target/release` two directories above the setup binary.
pub(crate) fn Fn(Name:&str, SearchPaths:&[PathBuf], TargetRelease:PathBuf) -> Source::Enum::Enum {
	SearchPaths
		.iter()
		.map(|SearchDirectory| SearchDirectory.join(Name))
		.find(|Candidate| Candidate.exists())
		.map(Source::Enum::Enum::Search)
		.or_else(|| {
			std::fs::metadata(&TargetRelease)
				.ok()
				.map(|_| Source::Enum::Enum::Build(TargetRelease))
		})
		.unwrap_or(Source::Enum::Enum::Downloaded)
}
