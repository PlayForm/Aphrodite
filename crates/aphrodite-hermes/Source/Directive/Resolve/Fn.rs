//! Resolution of the directory the embedded directives are provisioned into.
//!
//! This is the WHERE half of provisioning, kept apart from the write so the
//! precedence chain can be tested without touching a filesystem. The chain
//! exists because three different actors pick a runtime home - the FFI
//! caller, the core loader's env overrides, and the shared
//! `aphrodite::Home::Runtime::Fn::Fn` decision - and they must never disagree:
//! a directive written into a shadow home is invisible to the loader that
//! reads the canonical one (issue 40).

use std::path::PathBuf;

/// Resolve the target directives directory for provisioning.
///
/// Precedence (first non-empty wins):
///   1. `home_param` - explicit caller-supplied home (the FFI argument);
///      directives go to `<home_param>/directives/`.
///   2. `$APHRODITE_DIRECTIVES_DIR` - the directive loader's exact-directory
///      override (core crate candidate 0); materialize straight into it.
///   3. `$APHRODITE_HOME` - home-level override; directives go to
///      `<APHRODITE_HOME>/directives/`.
///   4. The shared runtime-home resolution (`aphrodite::Home::Runtime::Fn::Fn`:
///      `$HERMES_HOME`/aphrodite, then `$HOME/.hermes/aphrodite`, then the
///      platform user home) - one decision with the engine binary and the
///      plugin shim, so directives can never land in a second, shadow home
///      when `HERMES_HOME != $HOME/.hermes` (issue 40).
///   5. `.` - degraded fallback with a warning (never fail).
///
/// Returns `(directives_dir, warnings)`.
///
/// It is not a validator: a caller-supplied `home_param` is taken verbatim,
/// never checked for existence, so provisioning reports a create failure as a
/// warning instead of refusing the call.
pub(crate) fn Fn(home_param:&str) -> (PathBuf, Vec<String>) {
	let mut warnings:Vec<String> = Vec::new();

	// The three overrides, in precedence order. Each is resolved to its
	// directives directory (or `None` when unset/blank) so the chain below is
	// one `match` on the decision rather than three early returns.
	let explicit_home = (!home_param.is_empty()).then(|| PathBuf::from(home_param).join("directives"));
	let directive_directory = std::env::var("APHRODITE_DIRECTIVES_DIR")
		.ok()
		.filter(|dir| !dir.trim().is_empty())
		.map(PathBuf::from);
	let home_override = std::env::var("APHRODITE_HOME")
		.ok()
		.filter(|home| !home.trim().is_empty())
		.map(|home| PathBuf::from(home).join("directives"));

	match (explicit_home, directive_directory, home_override) {
		(Some(dir), ..) => (dir, warnings),
		(None, Some(dir), _) => (dir, warnings),
		(None, None, Some(dir)) => (dir, warnings),
		(None, None, None) => {
			match aphrodite::Home::Runtime::Opt::Fn::Fn() {
				Some(home) => (home.join("directives"), warnings),
				None => {
					warnings.push(
						"neither $HOME nor $HERMES_HOME nor $APHRODITE_HOME nor $APHRODITE_DIRECTIVES_DIR is set; \
						 using current directory"
							.into(),
					);

					(PathBuf::from(".").join("directives"), warnings)
				},
			}
		},
	}
}
