//! Configured-template vs fallback dispatch.
//!
//! One decision, in one place: the grammar the registration publishes decides
//! how the marker is rendered, and the fallback is reached through the named
//! `Grammar::Canonical` variant rather than through an absent format. The debug
//! counter and the verbose timing line live here too, so "how many markers did
//! this session render and how long did it take" is answered at the single
//! point every marker passes through.

use super::{
	Fallback::Fn as Fallback,
	Grammar::Enum::Enum as Grammar,
	Registration::With::Fn::Fn as With,
	Vars::Fn as Substitute,
};

/// Dispatch marker rendering: a registered `[templates.marker] format` wins, the
/// canonical three-part grammar is the fallback.
///
/// External contract: whichever grammar is in force, the returned string is the
/// marker block a consumer parses - `<<<CCR:{hash}|{type}|{size}>>>` followed by
/// the preview line, so an empty `Preview` still renders a preview line.
///
/// The registration is read once, inside one closure, so the format and the hint
/// this marker is rendered with come from the same publication - and neither is
/// cloned on the way out of it.
pub(super) fn Fn(Preview:&str, Type:&str, Meta:&str, Center:Option<&str>, Hash:&str, Size:usize) -> String {
	// DEV-ONLY debug instrumentation (crate::Debug): marker-render counter +
	// verbose line at APHRODITE_DEBUG=2 (captured only when verbose).
	let T0 = (crate::Debug::Level::Get::Fn::Fn() >= 2).then(std::time::Instant::now);
	let Out = With(|Registered| {
		let Hint = Registered.Hint.as_deref().unwrap_or("");
		match &Registered.Grammar {
			Grammar::Template(Format) => {
				let Number = Size.to_string();
				let Segment = Center.map(|Center| format!(";center={Center}")).unwrap_or_default();
				let Vars = [
					("preview", Preview),
					("type", Type),
					("meta", Meta),
					("center", Center.unwrap_or("")),
					("center_seg", Segment.as_str()),
					("hash", Hash),
					("size", Number.as_str()),
					("hint", Hint),
				];
				Substitute(Format, &Vars)
			},
			Grammar::Canonical => Fallback(Preview, Type, Meta, Center, Hash, Size, Hint),
		}
	});
	crate::Debug::Count::Bump::Fn::Fn(crate::Debug::Count::Enum::Enum::TemplateRenders);
	T0.into_iter().for_each(|T0| {
		crate::Debug::Log::Verbose::Fn::Fn(
			"marker_render",
			&[
				("hash", Hash.to_string()),
				("type", Type.to_string()),
				("size", Size.to_string()),
				("elapsed_us", T0.elapsed().as_micros().to_string()),
			],
		);
	});
	Out
}
