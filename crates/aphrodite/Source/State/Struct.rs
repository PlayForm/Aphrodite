//! The session state struct.
//!
//! `AphroditeState` is a **composition**, not a bag of fields: one sub-struct
//! per domain, each in the directory that owns it. The reason is authority, not
//! tidiness - a function that records a marker needs the catalog and nothing
//! else, so it takes `&mut Marker::Struct`; the compiler then refuses it the
//! store, the turn counter and the configuration instead of trusting a reader to
//! notice it never touched them.
//!
//! A field's path is also its namespace. `Store::Struct::InlineStoreBytes` says
//! *store* twice, so inside the sub-struct each field is one word: the domain
//! segment carries the qualifier the field used to repeat.
//!
//! | Domain          | Owns                                                        |
//! | --------------- | ----------------------------------------------------------- |
//! | `Store`         | the inline content store: map, LRU order, both bounds       |
//! | `Marker`        | the recent-marker catalog and its delta counter             |
//! | `Event`         | the per-tool-call telemetry ring                            |
//! | `File`          | the referenced-files list, its tool set, its delta counter  |
//! | `Turn`          | the turn counter, the scan cursor, the conversation index   |
//! | `ChainSplit`    | the split switch, its adaptive bounds, the consequence ledger |
//! | `Directive`     | loaded/active/ephemeral directives and the manual latch     |
//! | `PollWorker`    | the background-task deque and its master switch             |
//! | `Configuration` | every value mirrored out of `aphrodite.toml`                |
//!
//! No `impl` lives here. The operations live in the domain directory that owns
//! the data they touch, implemented on that domain's `Struct`.

use crate::State::{
	ChainSplit::Struct::Struct as ChainSplit, Configuration::Struct::Struct as Configuration,
	Directive::Struct::Struct as Directive, Event::Struct::Struct as Event,
	File::Struct::Struct as File, Marker::Struct::Struct as Marker,
	PollWorker::Struct::Struct as PollWorker, Store::Struct::Struct as Store,
	Turn::Struct::Struct as Turn,
};

/// Session state - one per loaded dylib instance.
///
/// `Clone` enables the hermes bridge's per-session registry: each new
/// session's state is a clone of a config-seeded template, so subagent
/// spawns inherit the same thresholds/directives without re-reading config.
/// Cloning is a deep copy of the (empty at seed time) CCR stores.
#[derive(Clone)]
pub struct AphroditeState {
	pub Store:Store,
	pub Marker:Marker,
	pub Event:Event,
	pub File:File,
	pub Turn:Turn,
	pub ChainSplit:ChainSplit,
	pub Directive:Directive,
	pub PollWorker:PollWorker,
	pub Configuration:Configuration,
}
