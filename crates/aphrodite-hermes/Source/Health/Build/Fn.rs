//! The build stamp, read from the compile-time environment.

use crate::Health::Build::Struct;

/// The stamp of the image that is running.
///
/// Every value but the version is `option_env!`, because the build script is the
/// only thing that can produce them and it degrades (a build with no `.git`, a
/// build script that could not run `git`) rather than failing the build. The
/// version always exists - cargo supplies it - so the stamp is never empty, and
/// `Struct::identity` is what says whether the rest of it amounts to a build
/// identity.
pub(crate) fn Fn() -> Struct::Struct {
	Struct::Struct {
		Version:env!("CARGO_PKG_VERSION"),
		Commit:option_env!("APHRODITE_HERMES_GIT_HASH"),
		Built:option_env!("APHRODITE_HERMES_BUILD_DATE"),
		Target:option_env!("APHRODITE_HERMES_TARGET"),
		Profile:option_env!("APHRODITE_HERMES_PROFILE"),
	}
}
