//! Classify a byte count by magnitude.

use super::super::{Kilobyte::Const::Const as Kilobyte, Megabyte::Const::Const as Megabyte, Unit};

/// Pick the unit a byte count is rendered in.
///
/// An ordered threshold ladder over one scalar, which is why the arms are guards: the
/// three ranges are mutually exclusive by construction and each arm binds the count it
/// classifies, so no branch has to re-read the value it was chosen by. The alternative
/// - a chain of `then_some(...).unwrap_or_else(...)` - is `clippy::obfuscated_if_else`
///   and reads worse for the same decision.
pub(crate) fn Fn(Bytes:usize) -> Unit::Enum::Enum {
	match Bytes {
		Bytes if Bytes >= Megabyte => Unit::Enum::Enum::Megabytes(Bytes),
		Bytes if Bytes >= Kilobyte => Unit::Enum::Enum::Kilobytes(Bytes),
		Bytes => Unit::Enum::Enum::Bytes(Bytes),
	}
}
