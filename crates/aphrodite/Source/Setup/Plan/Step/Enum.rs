//! The ordered operations a setup run performs.

/// One installation step.
///
/// The order these are produced in is the install contract, and it used to be the
/// order of statements in a 225-line function: a reader had to scroll to know what
/// ran before what, and a step that was conditional was a `match bool` in the middle
/// of the sequence.
///
/// As a value the order is the order of the plan's `Steps` list, which a caller can
/// read (or print) before anything happens, and a conditional step is a step that is
/// either in the list or not.
#[derive(Clone, Copy, Debug)]
pub(crate) enum Enum {
	/// Create the runtime home and its `binaries/` directory.
	Create,
	/// Take the plugin directory, unless a catalog install owns it.
	Prepare,
	/// Copy the running binary into `binaries/`.
	Binary,
	/// Copy or download both dylibs into `binaries/`.
	Dylib,
	/// Write `aphrodite.toml` - absent, or `--force`.
	Configuration,
	/// Write the generated `plugin.yaml` manifest.
	Manifest,
	/// Write the `__init__.py` loader.
	Loader,
	/// Pin the crate version into `BINARY_VERSION`.
	Pin,
	/// Run `hermes plugins enable aphrodite`.
	Register,
}
