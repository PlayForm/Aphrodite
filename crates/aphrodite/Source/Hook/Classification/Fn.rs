//! Resolve the reported type and the preview source.

use super::{
	Detect::Fn::Fn as Detect,
	Hinted::Fn::Fn as Hinted,
	Refine::Fn::Fn as Refine,
	Source::Enum::Enum as Source,
	Struct::Struct,
};

/// Resolve the reported `type`, the content the preview is built from, and how
/// the type was decided.
///
/// Shared by the tool-result and terminal-output paths (F14): both resolve an
/// optional classify pair, falling back to Aphrodite's semantic detector with a
/// single terminal override, so the reported `type` AND the preview both carry
/// the high-signal shape.
///
/// The three sources are alternatives in priority order, and each is tried only
/// when the previous one answered nothing - a caller-supplied pair wins
/// outright and is NOT re-detected, because the caller already looked underneath
/// a wrapper this layer cannot see through, and second-guessing it is how the
/// two paths would fork.
///
/// `TerminalHint` enables the override: the tool path passes `false`, the
/// terminal path `true`.
pub(crate) fn Fn<'a>(Content:&'a str, Classify:Option<(&'a str, &'a str)>, TerminalHint:bool) -> Struct<'a> {
	Classify
		.map(|(ClassifyContent, Type)| {
			Struct { Type:Type.to_string(), Content:ClassifyContent, Source:Source::Supplied }
		})
		.or_else(|| Hinted(Content, TerminalHint))
		.unwrap_or_else(|| Refine(Detect(Content)))
}
