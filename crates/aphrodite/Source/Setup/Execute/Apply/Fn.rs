//! One step of the plan.

use crate::{
	Configuration::CLI::Setup::Struct::Struct as SetupArgs,
	Setup::{
		Dylib::Copy::Fn::Fn as CopyDylibs,
		Execute::{Binary, Configuration, Directory, Failure, Plugin, Registration, Report},
		Plan::{self, Step},
		Write::{BinaryVersion, InitPy, PluginYAML},
	},
};

/// Carry out one named step, recording it in `Report` only when it succeeded.
///
/// The report is what a failure carries: `map_err` moves the completed prefix out
/// of the report and into the failure, so the caller can say which artifacts are
/// on disk without this function having to thread a second accumulator.
pub(crate) fn Fn(
	Report:&mut Report::Struct::Struct,
	Step:&Step::Enum::Enum,
	Plan:&Plan::Struct::Struct,
	Args:&SetupArgs,
) -> Result<(), Failure::Struct::Struct> {
	let Outcome = match Step {
		Step::Enum::Enum::Create => Directory::Fn::Fn(&Plan.Context),
		Step::Enum::Enum::Prepare => Plugin::Directory::Fn::Fn(Plan),
		Step::Enum::Enum::Binary => Binary::Fn::Fn(&Plan.Context),
		Step::Enum::Enum::Dylib => CopyDylibs(&Plan.Context),
		Step::Enum::Enum::Configuration => Configuration::Fn::Fn(&Plan.Context, Args),
		Step::Enum::Enum::Manifest => PluginYAML::Fn(&Plan.Context, Args),
		Step::Enum::Enum::Loader => InitPy::Fn(&Plan.Context),
		Step::Enum::Enum::Pin => BinaryVersion::Fn(&Plan.Context),
		Step::Enum::Enum::Register => Registration::Fn::Fn().map(|Registered| Report.Registration = Some(Registered)),
	};

	Outcome.map_err(|Error| Failure::Struct::Struct { Completed:std::mem::take(&mut Report.Completed), Error })?;

	Report.Completed.push(*Step);

	Ok(())
}
