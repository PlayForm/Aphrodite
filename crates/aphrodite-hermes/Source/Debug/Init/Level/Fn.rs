//! The filter default when neither `RUST_LOG` nor `APHRODITE_LOG` names one.

/// The filter spec to fall back to when neither env var is set.
///
/// Reads the level the engine captured at config load rather than the
/// environment again: a second read could disagree with the level the debug
/// file and the flag toggle are gated on, and a log whose filter disagrees with
/// the feature it logs is worse than no log.
///
/// The three levels are the engine's own encoding (`0` -> warn, `1` -> debug,
/// `2` and above -> trace); an unknown value takes the widest of them, because a
/// level this crate cannot interpret must not silently narrow the log.
pub(crate) fn Fn() -> &'static str {
	match aphrodite::Debug::Level::Get::Fn::Fn() {
		0 => "warn",
		1 => "debug",
		_ => "trace",
	}
}
