//! The directives baked into the binary.

/// The embedded directive set: identifier and body, in the order they are advertised.
///
/// The identifiers are `BuiltinDirective/<id>.md` file stems and they are data keys, not
/// names: they are surfaced to the model and addressed by
/// `aphrodite_directive("add", ...)`, so they stay lowercase and unchanged while the
/// directory around them is PascalCase.
pub(crate) const Const:[(&str, &str); 7] = [
	("focus", include_str!("../../BuiltinDirective/focus.md")),
	("foresight", include_str!("../../BuiltinDirective/foresight.md")),
	("ccr-handling", include_str!("../../BuiltinDirective/ccr-handling.md")),
	("cleanup", include_str!("../../BuiltinDirective/cleanup.md")),
	("explore", include_str!("../../BuiltinDirective/explore.md")),
	("lazy", include_str!("../../BuiltinDirective/lazy.md")),
	("lazy-eval", include_str!("../../BuiltinDirective/lazy-eval.md")),
];
