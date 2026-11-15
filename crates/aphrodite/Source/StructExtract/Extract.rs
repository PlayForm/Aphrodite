//! The structure-preview entry point.
//!
//! Owns the one budget a preview gets and the category order it is spent in. Both belong here rather
//! than in a language leaf: the budget has to be created once per preview so the languages cannot each
//! claim their own, and the order is the policy's, not a recogniser's.

use std::collections::HashMap;

use crate::StructExtract::{
	Budget::{Const::Const as Floor, Struct::Struct as Budget},
	Detect::Fn as AutoDetect,
	Policy::Fn::Fn as Categories,
	Scan::Fn::Fn as Scan,
	Summary::Struct::Struct as Summary,
};

/// Extract code structure from source content, as category to signature lines.
///
/// A non-empty `Language` is trusted as given; an empty one means "ask `Detect`", which is what the
/// preview builder passes for a blob whose type was never recorded. Only categories that produced at
/// least one line appear in the map (`fns`, `structs`, `classes`, `types`, `traits`, `impls`, depending
/// on the language), so an absent key means "none found" rather than "not asked for".
///
/// It is not a parser and it does not verify that the content really is the language it was told: an
/// unrecognised tag yields an empty map rather than an error, because a preview with no structure is a
/// degraded marker, not a failure of the compression that produced it. The budget is created here and
/// shared by every category of the call, so the tail categories of a long file are dropped once it is
/// spent.
pub fn Fn(Content:&str, Language:&str) -> HashMap<String, Vec<String>> {
	let Tag = match Language.is_empty().then(|| AutoDetect(Content)) {
		Some(Tag) => Tag,
		None => Language.to_string(),
	};

	let mut Budget = Budget::new(Floor);
	let mut Summary = Summary::new();

	Categories(&Tag).iter().for_each(|Category| {
		let Lines = Scan(Content, &mut Budget, Category.Recognize);

		Summary.Push(Category.Key, Lines);
	});

	Summary.IntoMap()
}
