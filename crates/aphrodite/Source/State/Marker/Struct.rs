//! The recent-marker catalog's own fields.
//!
//! Split out of `AphroditeState` so a function that only reads the catalog can
//! take `&Marker::Struct`: rendering a catalog, deciding whether new markers
//! arrived this turn, and recording one all need the ring and the delta counter,
//! and nothing else the session carries.
//!
//! `Entry` is the item; this struct is the collection and its bookkeeping.

use std::collections::VecDeque;

use super::Entry::Struct::Struct as Entry;

/// The marker catalog: the bounded ring of recorded markers and the counter the
/// catalog renderer uses to emit a delta line only when something is new.
#[derive(Clone)]
pub struct Struct {
	/// Recent CCR markers for catalog: [{hash, type, size, preview, turn}].
	/// `VecDeque` (fix 6): `Record` pushes back and pops the front
	/// while over `Marker::Const::Cap`, so the oldest marker always sits at
	/// index 0.
	pub RecentMarkers:VecDeque<Entry>,
	/// Number of markers the last time catalog_summary rendered, so we emit a
	/// delta line only when new markers arrived this turn. Zero-initialized;
	/// reset on session start. Stops the prompt-cache-poisoning repetition of
	/// the same 5 previews every turn.
	pub LastEmittedMarkerCount:usize,
}
