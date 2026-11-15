//! Scan content for one category's signatures.

use std::ops::ControlFlow;

use crate::StructExtract::{
	Budget::Struct::Struct as Budget,
	Declaration::Enum::Enum as Declaration,
	Render::Fn::Fn as Render,
};

/// Collect the signatures one category's recognisers find, until the budget is spent.
///
/// The recognisers are tried in order per line and the first rendered signature wins, which is what
/// keeps `fns` from reporting a line twice when a category is filled by two shapes. The `scan` ends the
/// iterator the moment the budget breaks, so the content is not walked past the point where nothing
/// more can be admitted - and `Break` is a completion, not an error, so no sentinel value is invented.
///
/// It is not a formatter: every line arrives already rendered and already capped.
pub(crate) fn Fn(Content:&str, Budget:&mut Budget, Recognizers:&[fn(&str) -> Declaration]) -> Vec<String> {
	let mut Lines:Vec<String> = Vec::new();

	Content
		.lines()
		.filter_map(|Line| Recognizers.iter().find_map(|Recognize| Render(Recognize(Line.trim()))))
		.scan((), |(), Signature| {
			match Budget.Charge(Signature.len()) {
				ControlFlow::Break(()) => None,
				ControlFlow::Continue(()) => Some(Signature),
			}
		})
		.for_each(|Signature| Lines.push(Signature));

	Lines
}
