//! The seed state every session starts from.
//!
//! The hermes bridge clones a config-seeded `AphroditeState` per session, so
//! this is the state a session has before `Apply::Startup::Fn` runs over it.
//! Each domain seeds itself (`Store::Default`, `Marker::Default`, ...) so a
//! domain's defaults live with the domain that owns them and this file stays a
//! composition, not a second copy of the field list.
//!
//! Two values are deliberately NOT the shipped configuration:
//! `ChainSplit::Enable` and `Configuration::AutoExpand` start false in their own
//! `Default.rs` so the shipped default is reachable through the config layer
//! alone, and a missing config can never turn either feature on.

use super::{
	ChainSplit::Struct::Struct as ChainSplit, Configuration::Struct::Struct as Configuration,
	Directive::Struct::Struct as Directive, Event::Struct::Struct as Event,
	File::Struct::Struct as File, Marker::Struct::Struct as Marker,
	PollWorker::Struct::Struct as PollWorker, Store::Struct::Struct as Store,
	Struct::AphroditeState, Turn::Struct::Struct as Turn,
};

impl Default for AphroditeState {
	fn default() -> Self {
		Self {
			Store:Store::default(),
			Marker:Marker::default(),
			Event:Event::default(),
			File:File::default(),
			Turn:Turn::default(),
			ChainSplit:ChainSplit::default(),
			Directive:Directive::default(),
			PollWorker:PollWorker::default(),
			Configuration:Configuration::default(),
		}
	}
}
