//! Resolution of the directory the embedded directives are provisioned into.
//!
//! This is the WHERE half of provisioning, kept apart from the write so the
//! precedence chain can be tested without touching a filesystem. The chain
//! exists because three different actors pick a runtime home - the FFI
//! caller, the core loader's env overrides, and the shared
//! `aphrodite::Home::Runtime::Fn::Fn` decision - and they must never disagree:
//! a directive written into a shadow home is invisible to the loader that
//! reads the canonical one (issue 40).
//!
//! Each candidate is its own leaf and the chain is `or_else`, so a candidate is
//! only evaluated when every candidate above it missed. The tuple this replaced
//! read `$APHRODITE_DIRECTIVES_DIR` and `$APHRODITE_HOME` even when the caller
//! had already supplied a home.

use std::path::PathBuf;

use crate::Directive::Resolve::{Explicit, Home, Loader, Runtime};

/// Resolve the target directives directory for provisioning.
///
/// Precedence (first non-empty wins):
///   1. `HomeParam` - explicit caller-supplied home (the FFI argument);
///      directives go to `<HomeParam>/directives/`.
///   2. `$APHRODITE_DIRECTIVES_DIR` - the directive loader's exact-directory
///      override (core crate candidate 0); materialize straight into it.
///   3. `$APHRODITE_HOME` - home-level override; directives go to
///      `<APHRODITE_HOME>/directives/`.
///   4. The shared runtime-home resolution (`aphrodite::Home::Runtime::Fn::Fn`:
///      `$HERMES_HOME`/aphrodite, then `$HOME/.hermes/aphrodite`, then the
///      platform user home) - one decision with the engine binary and the
///      plugin shim, so directives can never land in a second, shadow home
///      when `HERMES_HOME != $HOME/.hermes` (issue 40).
///   5. `./directives` - degraded fallback with a warning (never fail).
///
/// Returns `(directives_dir, warnings)`.
///
/// It is not a validator: a caller-supplied `HomeParam` is taken verbatim,
/// never checked for existence, so provisioning reports a create failure as a
/// warning instead of refusing the call.
pub(crate) fn Fn(HomeParam:&str) -> (PathBuf, Vec<String>) {
	let Resolved = Explicit::Fn::Fn(HomeParam)
		.or_else(Loader::Fn::Fn)
		.or_else(Home::Fn::Fn)
		.or_else(Runtime::Fn::Fn);

	Resolved.map_or_else(
		|| {
			(
				PathBuf::from(".").join("directives"),
				vec![
					"neither $HOME nor $HERMES_HOME nor $APHRODITE_HOME nor $APHRODITE_DIRECTIVES_DIR is set; using \
					 current directory"
						.into(),
				],
			)
		},
		|Dir| (Dir, Vec::new()),
	)
}
