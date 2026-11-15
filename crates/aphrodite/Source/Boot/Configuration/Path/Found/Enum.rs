//! Whether a configuration file was found.

/// The result of looking for a configuration file.
///
/// A `bool` said the same thing and admitted no third state, so every caller had
/// to remember which way round it went ("`true` means use the file"). This is
/// that decision with its cases named: `Load` matches it to choose between
/// parsing the document and parsing the command line, and there is no way to
/// express "found a file but parse the CLI" at all.
#[derive(Clone, Copy)]
pub(crate) enum Enum {
	/// A file exists at the resolved path and must be parsed.
	Present,
	/// Nothing was found, so the command line is the configuration.
	Absent,
}
