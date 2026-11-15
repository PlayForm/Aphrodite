//! Match one normalised line against the nine declaration shapes.

use crate::Stage2::Reduce::Code::{Matcher::Enum::Enum, Normalised::Struct::Struct as Normalised};

/// Recognise a declaration shape, or `None` when the line declares nothing.
///
/// The prefix lists are exactly the ones the previous hand-rolled matcher accepted - including
/// the bare `function ` form, which its `Pattern` string did not spell but its prefix test did, so
/// this preserves behaviour rather than the pattern's intent. The shapes are mutually exclusive by
/// prefix, so the first match is the only match.
///
/// It is not a parser: nothing after the prefix is read, because the reducer only reports that a
/// declaration of this kind exists on this line.
pub(crate) fn Fn(Normalised:&Normalised<'_>) -> Option<Enum> {
	let Lower = Normalised.Lower();

	let Function = Lower.starts_with("fn ")
		|| Lower.starts_with("pub fn ")
		|| Lower.starts_with("async fn ")
		|| Lower.starts_with("pub async fn ");
	let Struct = Lower.starts_with("struct ") || Lower.starts_with("pub struct ");
	let Enumeration = Lower.starts_with("enum ") || Lower.starts_with("pub enum ");
	let Trait = Lower.starts_with("trait ") || Lower.starts_with("pub trait ");
	let Impl = Lower.starts_with("impl ") || Lower.starts_with("impl<");
	let Def = Lower.starts_with("def ");
	let Class = Lower.starts_with("class ");
	let Func = Lower.starts_with("func ");
	let Exported = Lower.starts_with("export function ")
		|| Lower.starts_with("export async function ")
		|| Lower.starts_with("function ");

	Function
		.then_some(Enum::Fn)
		.or_else(|| Struct.then_some(Enum::Struct))
		.or_else(|| Enumeration.then_some(Enum::Enumeration))
		.or_else(|| Trait.then_some(Enum::Trait))
		.or_else(|| Impl.then_some(Enum::Impl))
		.or_else(|| Def.then_some(Enum::Def))
		.or_else(|| Class.then_some(Enum::Class))
		.or_else(|| Func.then_some(Enum::Func))
		.or_else(|| Exported.then_some(Enum::Function))
}
