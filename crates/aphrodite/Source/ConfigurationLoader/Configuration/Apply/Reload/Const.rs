//! The reload pass: every section a live session has not taken ownership of.

use crate::{
	ConfigurationLoader::Configuration::{
		Apply::Section::{ChainSplit, Debug, Engine, Flow, Prompt, Runtime, Template},
		Struct::Struct as Configuration,
	},
	State::Struct::AphroditeState,
};

/// Every section a reload may touch - `Startup/Const` without `Directive`.
///
/// A live session owns its directive set: `aphrodite_directive` swaps, adds and removes
/// entries at runtime, so a reload has no authority to re-discover the directory and
/// reseed `ActiveDirectives` from `[directives] active`. The absence of `Directive` here
/// IS that fix - there is nothing to snapshot and restore afterwards, because the reload
/// path cannot reach the field at all.
pub(crate) const Reload:&[fn(&Configuration, &mut AphroditeState)] = &[
	Engine::Apply,
	Flow::Apply,
	Runtime::Apply,
	Debug::Apply,
	ChainSplit::Apply,
	Template::Apply,
	Prompt::Apply,
];
