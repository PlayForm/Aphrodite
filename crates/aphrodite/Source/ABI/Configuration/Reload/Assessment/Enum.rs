//! The four situations a reload can be in, as one value.

/// What a reload found on disk.
///
/// Four variants, because four situations have four different consequences and
/// collapsing any two of them is how a reload stops being diagnosable:
///
/// - `Unreadable` - no path was given, or the file could not be read. This is
///   the normal search miss: nothing changes and **no diagnostic is recorded**.
///   It is not a failure and must not clear or set `Configuration::Error`.
/// - `Invalid` - the file was read and is not valid TOML. The previous value
///   stands, a warning goes to the log surface, and the diagnostic is recorded
///   on the session so `AphroditeStats` self-diagnoses.
/// - `Ready` - the file parsed and carries `[compression] context_engine`. The
///   value is applied and no diagnostic is recorded.
/// - `Omitted` - the file parsed but omits the key. Nothing changes; in
///   particular a key that is absent from the file is not a statement that the
///   current value is wrong, so no diagnostic is recorded either.
///
/// Only `Invalid` ever writes `Configuration::Error`, and only `Invalid` ever
/// reports one back. A successful reload therefore leaves an earlier parse
/// failure's diagnostic in place rather than clearing it: the reload answered
/// the key it was asked about, and saying nothing about a different file's
/// earlier failure is the honest answer, not a stale one.
pub(crate) enum Enum {
	Unreadable,
	Invalid { Diagnostic:String },
	Ready { Enabled:bool },
	Omitted,
}
