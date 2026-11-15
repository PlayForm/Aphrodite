//! Byte-count rendering shared by both catalog surfaces.

use super::{Classify, Kilobyte::Const::Const as Kilobyte, Megabyte::Const::Const as Megabyte, Unit};

/// Render a byte count as `B`, `KB` or `MB`.
///
/// Binary units (1024-based), one decimal for megabytes and none for kilobytes - the
/// same shape the catalog table and the `total_saved_human` field have always
/// reported, so a change here changes what an agent reads in both places at once.
///
/// The unit is decided by [`Classify::Fn::Fn`] and rendered here, so the divisor and
/// the suffix cannot disagree: `Megabytes` divides by a megabyte, `Kilobytes` by a
/// kilobyte, and the number in each arm is the number the classifier measured.
pub(crate) fn Fn(Bytes:usize) -> String {
	match Classify::Fn::Fn(Bytes) {
		Unit::Enum::Enum::Megabytes(Bytes) => format!("{:.1}MB", Bytes as f64 / Megabyte as f64),
		Unit::Enum::Enum::Kilobytes(Bytes) => format!("{:.0}KB", Bytes as f64 / Kilobyte as f64),
		Unit::Enum::Enum::Bytes(Bytes) => format!("{}B", Bytes),
	}
}
