//! The startup pass: every section, in the one order that works.

use crate::{
	ConfigurationLoader::Configuration::{
		Apply::Section::{ChainSplit, Debug, Directive, Engine, Flow, Prompt, Runtime, Template},
		Struct::Struct as Configuration,
	},
	State::Struct::AphroditeState,
};

/// Every section, in the order the configuration has always been applied.
///
/// `Directive` runs before `Template` and `Prompt`, and a few sections seed values the
/// next one reads (`ChainSplitFloor` from `ChainSplitMinimumSegments`, `ActiveDirectives`
/// from the loaded set), so the order is a contract rather than a preference.
///
/// This list is the whole difference between a startup pass and a reload: `Reload/Const`
/// is this list without `Directive`. The order is data so the two can be compared, and so
/// a section cannot be applied in one pass and silently skipped in the other by accident.
pub(crate) const Sequence:&[fn(&Configuration, &mut AphroditeState)] = &[
	Engine::Apply,
	Flow::Apply,
	Runtime::Apply,
	Debug::Apply,
	ChainSplit::Apply,
	Directive::Apply,
	Template::Apply,
	Prompt::Apply,
];
