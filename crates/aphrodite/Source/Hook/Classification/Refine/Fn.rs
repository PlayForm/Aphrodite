//! The preview-aware upgrade of a detected type.

use crate::Hook::Classification::{Source::Enum::Enum as Source, Struct::Struct};

/// Upgrade a generic classification using Aphrodite's own detector.
///
/// Terminal output is very often a git status / ls / test / grep dump, so the
/// reported type is refined from the content's shape rather than left generic -
/// the preview is only high-signal if the type names the shape.
///
/// The comparison against the detector's answer is converted into an `Option`
/// here and nowhere else: `Some` means "the refinement decided something", `None`
/// means the detector's answer stands. That is the one place a boolean exists, so
/// every consumer matches a source instead of re-testing two strings.
pub(crate) fn Fn<'a>(Detected:Struct<'a>) -> Struct<'a> {
	let Inp = crate::Preview::Input::Struct::Struct::new(Detected.Content)
		.unwrap_or_else(|| crate::Preview::Input::Struct::Struct::Empty(Detected.Content));
	let Refined = crate::Preview::Type::Fn::Fn(&Detected.Type, &Inp).into_owned();
	let Refinement = (Refined != Detected.Type).then_some(Refined);

	let (Type, Source) = Refinement.map_or((Detected.Type, Source::Detected), |Refined| {
		(Refined, Source::PreviewRefinement)
	});

	Struct { Type, Content:Detected.Content, Source }
}
