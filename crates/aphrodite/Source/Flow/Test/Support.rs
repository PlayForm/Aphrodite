//! Fixtures shared by the Flow test areas.
//!
//! `AphroditeState` is a plain struct with public fields, so a fixture builds
//! the smallest state that exercises the assembler rather than a full session.
//! Everything a test needs to observe is therefore set explicitly at the call
//! site, and no fixture hides a default that a later change could move.

use crate::Directive::Model::Struct as Directive;
use crate::State::Struct::AphroditeState;
use crate::State::Marker::Struct::MarkerEntry;

/// A state carrying one directive, active and ready to render.
pub(super) fn StateWithDirective(name:&str, body:&str) -> AphroditeState {
	let mut s = AphroditeState::default();
	s.Directives
		.insert(name.into(), Directive { name:name.into(), Content:body.into() });
	s.ActiveDirectives = vec![name.into()];
	s
}

/// Record one marker of a fixed size, so catalog lengths are predictable.
pub(super) fn AddMarker(s:&mut AphroditeState, hash:&str, Turn:usize) {
	s.RecordMarker(MarkerEntry {
		hash:hash.into(),
		CCRType:"text".into(),
		Size:100,
		Preview:"[text] some preview content here".into(),
		Turn,
		Center:None,
		meta:None,
	});
}

/// A state with a templates table holding exactly one prompt entry.
pub(super) fn StateWithPrompt(name:&str, Tmpl:&str) -> AphroditeState {
	let mut s = AphroditeState::default();
	s.Templates = Some(crate::State::TemplatesConfiguration::default());
	s.Templates.as_mut().unwrap().Prompts.insert(name.into(), Tmpl.into());
	s
}
