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
	CreateDirectories,
	/// Take the plugin directory, unless a catalog install owns it.
	PreparePluginDirectory,
	/// Copy the running binary into `binaries/`.
	InstallBinary,
	/// Copy or download both dylibs into `binaries/`.
	InstallDylibs,
	/// Write `aphrodite.toml` - absent, or `--force`.
	WriteConfiguration,
	/// Write the generated `plugin.yaml` manifest.
	WriteManifest,
	/// Write the `__init__.py` loader.
	WriteLoader,
	/// Pin the crate version into `BINARY_VERSION`.
	WriteVersionPin,
	/// Run `hermes plugins enable aphrodite`.
	RegisterPlugin,
}
