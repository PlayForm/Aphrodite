//! `Emit`: the same record, into the `tracing` stack.

/// Emit one already-formatted debug record as a `tracing` event.
///
/// The record is emitted on the `aphrodite::debug` target at `DEBUG`, so an
/// operator who set `APHRODITE_LOG=aphrodite=debug` (or `RUST_LOG`) sees the
/// engine's per-turn decisions in the SAME stream as the proxy's own spans and
/// warnings, in the same order they happened. At the default `info` filter the
/// event is dropped at the callsite and costs one interest check - which is
/// why this cannot turn a release build into a chatty one.
///
/// The line is passed as one field rather than re-expanded into per-key
/// fields on purpose: the file's format is the documented contract (one
/// `[stamp] [aphrodite-debug] lvl=.. event=.. k=v` line) and a second,
/// structurally different rendering of the same record would drift from it.
/// `sanitize_kv` has already flattened the newlines by the time this runs, so a
/// record can never break the event into two lines here either.
pub(crate) fn Fn(Level:u8, Line:&str) {
	tracing::event!(
		target:"aphrodite::debug",
		tracing::Level::DEBUG,
		debug_level = Level,
		record = %Line.trim_end(),
		"aphrodite debug record"
	);
}
