//! Which listing lines are entries at all.

/// True when a line is a listing entry rather than decoration.
///
/// `ls -l` prints a `total <N>` header that is not a filesystem entry, so it is
/// dropped - but only when the remainder is a number. A line that reads `total ` with
/// nothing after it, or `total abc`, is kept, because the old reader kept it and a
/// listing that literally contains such a line is what the model is being shown. It is
/// not an emptiness filter: blank lines are dropped, and that is the whole test.
pub(crate) fn Fn(Line:&str) -> bool {
	let Trimmed = Line.trim();

	match Trimmed.strip_prefix("total ") {
		Some(Rest) => Rest.is_empty() || !Rest.chars().all(|Character| Character.is_ascii_digit()),
		None => !Trimmed.is_empty(),
	}
}
