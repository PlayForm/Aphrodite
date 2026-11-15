//! One category of a language's structure.

use crate::StructExtract::Declaration::Enum::Enum as Declaration;

/// A wire category and the recognisers that fill it.
///
/// `Key` is the category string the model reads (`fns`, `structs`, `traits`, `impls`, `classes`,
/// `types`), and it lives here - in the policy - rather than in a recogniser, so a recogniser cannot
/// name a wire category. `Recognize` is a slice because one category can be recognised by more than
/// one shape: JavaScript's `fns` covers both `function` declarations and arrow bindings.
///
/// It is not a budget share: the budget is one counter shared by every category in the list.
pub(crate) struct Struct {
	pub Key:&'static str,
	pub Recognize:&'static [fn(&str) -> Declaration],
}
