//! The nine declaration shapes the code reducer recognises.

/// A recognised declaration shape.
///
/// This replaces a `&'static str` "pattern" that was examined for substrings like `fn\s+(\w+)`
/// and then dispatched to a prefix test - a string acting as an undocumented instruction enum,
/// where a typo in the pattern was a silent non-match rather than a compile error. The nine
/// variants are the nine rows that existed, and the label each one renders as is the same spelling
/// the previous array carried.
///
/// It is not a language: `Fn` and `Func` are different shapes because the previous matcher
/// distinguished them, even though both are function declarations.
pub(crate) enum Enum {
	Fn,
	Struct,
	Enumeration,
	Trait,
	Impl,
	Def,
	Class,
	Func,
	Function,
}

impl Enum {
	/// The `[kind]` label this shape renders with.
	///
	/// The spellings are the wire-visible ones: a reduction line reads `  [fn] pub fn Name(...)`,
	/// so changing a label here changes what the model reads.
	pub(crate) fn Kind(&self) -> &'static str {
		match self {
			Enum::Fn => "fn",
			Enum::Struct => "struct",
			Enum::Enumeration => "enum",
			Enum::Trait => "trait",
			Enum::Impl => "impl",
			Enum::Def => "def",
			Enum::Class => "class",
			Enum::Func => "func",
			Enum::Function => "function",
		}
	}
}
