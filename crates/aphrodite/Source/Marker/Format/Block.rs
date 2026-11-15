//! Configured-template vs fallback dispatch.
//!
//! One decision, in one place: if a `[templates.marker] format` string is
//! registered it wins, otherwise the canonical grammar is used. The debug
//! counter and the verbose timing line live here too, so "how many markers did
//! this session render and how long did it take" is answered at the single
//! point every marker passes through.

use super::{
	Const::{CurrentHint, RegisteredMarkerFormat},
	Fallback::RenderMarkerFallback,
	Vars::SubstituteVars,
};

/// Dispatch marker rendering: configured `[templates.marker] format` wins,
/// the canonical three-part grammar is the fallback.
pub(super) fn RenderMarkerBlock(
	Preview:&str,
	CCRType:&str,
	meta:&str,
	Center:Option<&str>,
	hash:&str,
	Size:usize,
) -> String {
	// DEV-ONLY debug instrumentation (crate::Debug): marker-render counter +
	// verbose line at APHRODITE_DEBUG=2 (captured only when verbose).
	let T0 = (crate::Debug::Level::Get::Fn::Fn() >= 2).then(std::time::Instant::now);
	let Hint = CurrentHint();
	let Out = match RegisteredMarkerFormat() {
		Some(fmt) => {
			let SizeS = Size.to_string();
			let CenterSeg = Center.map(|c| format!(";center={c}")).unwrap_or_default();
			let Vars = [
				("preview", Preview),
				("type", CCRType),
				("meta", meta),
				("center", Center.unwrap_or("")),
				("center_seg", CenterSeg.as_str()),
				("hash", hash),
				("size", SizeS.as_str()),
				("hint", Hint.as_str()),
			];
			SubstituteVars(&fmt, &Vars)
		},
		None => RenderMarkerFallback(Preview, CCRType, meta, Center, hash, Size, &Hint),
	};
	crate::Debug::Count::Bump::Fn::Fn(crate::Debug::Count::Enum::Enum::TemplateRenders);
	match T0 {
		Some(T0) => {
			crate::Debug::Log::Verbose::Fn::Fn(
				"marker_render",
				&[
					("hash", hash.to_string()),
					("type", CCRType.to_string()),
					("size", Size.to_string()),
					("elapsed_us", T0.elapsed().as_micros().to_string()),
				],
			)
		},
		None => (),
	}
	Out
}
