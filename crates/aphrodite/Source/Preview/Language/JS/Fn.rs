//! JavaScript / TypeScript evidence.

/// True when the content carries a JS declaration form AND a module form.
///
/// A function, a `const` binding or an arrow, plus an `import` or an `export`. It is not a
/// parser and it is not a tie-break: the language rules are tried in order and the first that
/// matches wins.
pub(crate) fn Fn(Content:&str) -> bool {
	(Content.contains("function ") || Content.contains("const ") || Content.contains("=> "))
		&& (Content.contains("import ") || Content.contains("export "))
}
