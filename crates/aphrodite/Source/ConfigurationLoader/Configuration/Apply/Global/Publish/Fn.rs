//! Writing a `Rendering` value into the three process globals, once.

use super::super::Rendering::Struct::Struct as Rendering;

/// Publish every process-global rendering setting from one document.
///
/// This is the only place the three globals are written, so "what does a load
/// publish" has one answer instead of four partial ones.
///
/// The template pair takes ONE registry write lock. `Template::Set` and
/// `Template::Family::Set` each take the lock themselves, so a caller that used both
/// published a family and a template table under two separate critical sections - a
/// reader between them renders one document's family against another's templates.
/// Publishing the pair here means a reader observes the previous pair or this one,
/// never a mix.
///
/// The preview cap is a single value of its own, so it needs no such pairing.
/// The marker grammar publishes its format and its hint as ONE registration
/// (`Marker::Format::Registration`), so a reader cannot render one document's
/// format against another's hint.
pub fn Fn(Rendering:&Rendering) {
	// The previous cap is only useful to a caller that wants to restore it; a
	// publication is not one, so the value is dropped explicitly rather than left as a
	// `must_use` statement.
	let _ = crate::Preview::State::Set::Fn(Rendering.PreviewMaximum);

	crate::Marker::Format::Registration::Set::Fn::Fn(
		Rendering.MarkerFormat.as_deref(),
		Rendering.MarkerHint.as_deref(),
		Rendering.MarkerHintEnabled,
	);

	let mut Registry = crate::Template::State::Fn::Fn()
		.write()
		.unwrap_or_else(std::sync::PoisonError::into_inner);

	Registry.Templates = Rendering.Templates.clone();
	Registry.Family = Rendering.Family.clone();
}
