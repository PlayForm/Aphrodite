//! `#include` directive predicate.

/// True for a C/C++ include directive (`#include <...>`, `#include "..."`,
/// `#include<...>`).
///
/// The `#include` prefix must be followed by `<` or `"` after optional
/// whitespace. It is not a preprocessor reader: no macro expansion and no
/// conditional-include awareness, so a line inside a disabled `#if 0` block
/// still counts. That is acceptable because the predicate only votes on
/// whether the content is code.
pub(crate) fn Fn(t:&str) -> bool {
	let Rest = match t.strip_prefix("#include") {
		Some(r) => r,
		None => return false,
	};
	let Rest = Rest.trim_start();
	Rest.starts_with('<') || Rest.starts_with('"')
}
