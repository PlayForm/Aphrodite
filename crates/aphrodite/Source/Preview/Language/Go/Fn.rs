//! Go evidence.

/// True when the content carries `func` or `package` AND a grouped import.
///
/// The grouped `import (` form is what distinguishes Go from a JavaScript file that happens to
/// declare a `func`-named binding. It is not a parser: nothing is tokenised.
pub(crate) fn Fn(Content:&str) -> bool {
	(Content.contains("func ") || Content.contains("package ")) && Content.contains("import (")
}
