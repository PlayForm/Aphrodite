//! A reduction that is known to be shorter than what it replaces.

/// A replacement that saved bytes.
///
/// The field is `pub(super)` rather than public, so the ONLY way to obtain a value of this type
/// is `Reduction::Accept::Fn` - which admits a candidate only when it is strictly shorter than
/// the original. "A reduction exists implies it is shorter" therefore holds by construction, and
/// no consumer needs to re-measure before replacing content.
///
/// It is not a validated string: nothing about the candidate's content is checked, because the
/// reducers are the ones that decide what a good reduction is.
pub(crate) struct Struct {
	pub(super) Content:String,
}

impl Struct {
	/// The accepted replacement, moved out.
	///
	/// Takes `self` by value so the string is moved rather than copied - a reduction is a
	/// whole payload and cloning it would defeat the point of having produced a smaller one.
	/// It is not a borrow: the value has no further use once its content has been handed on.
	pub(crate) fn Take(self) -> String { self.Content }
}
