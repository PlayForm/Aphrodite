//! The conversational-directive domain's own fields.
//!
//! Split out of `AphroditeState` so the directive verbs and the turn-context
//! assembler take `&mut Directive::Struct`: loading, activating, swapping and
//! resetting directives needs the loaded set, the active names, the ephemeral
//! nudges and the manual-mutation latch - not the store, the markers or the
//! configuration.

use std::collections::HashMap;

use super::Entry::Struct::Struct as Entry;

/// The directive set: everything loaded, the subset active, the ephemeral
/// nudges, and the turn a manual mutation last happened on.
///
/// `Default` is the seed: nothing loaded, nothing active, no latch.
#[derive(Clone, Default)]
pub struct Struct {
	/// All loaded directives (name → content).
	pub Loaded:HashMap<String, crate::Directive::Model::Struct>,
	/// Currently active directive names (the ones injected into context).
	pub Active:Vec<String>,
	/// Ephemeral (one-shot / TTL) directives - inline nudges that render once
	/// (or for a bounded number of turns) then self-purge (P3/T9). Distinct
	/// from `Directive::Active` (permanent-until-removed named entries).
	pub Ephemeral:Vec<Entry>,
	/// Turn number of the most recent MANUAL `AphroditeDirective` mutation
	/// (swap/add/remove/reset). Latches phase-aware auto-swaps out (P6); set by
	/// `Directive::Action::Fn` on any successful mutation.
	pub Manual:Option<usize>,
}
