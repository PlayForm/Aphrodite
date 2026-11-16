//! One step of the plan.

use crate::{
	Configuration::CLI::Setup::Struct::Struct as SetupArgs,
	Setup::{
		Dylib::Copy::Fn::Fn as CopyDylibs,
		Execute::{Binary, Configuration, Directory, Failure, Plugin, Registration, Report},
		Plan::{self, Step},
		Write::{
			BinaryVersion::Fn as WriteBinaryVersion,
			InitPy::Fn as WriteInitPy,
			PluginYAML::Fn as WritePluginYAML,
		},
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
		Step::Enum::Enum::CreateDirectories => Directory::Fn::Fn(&Plan.Context),
		Step::Enum::Enum::PreparePluginDirectory => Plugin::Directory::Fn::Fn(Plan),
		Step::Enum::Enum::InstallBinary => Binary::Fn::Fn(&Plan.Context),
		Step::Enum::Enum::InstallDylibs => CopyDylibs(&Plan.Context),
		Step::Enum::Enum::WriteConfiguration => Configuration::Fn::Fn(&Plan.Context, Args),
		Step::Enum::Enum::WriteManifest => WritePluginYAML(&Plan.Context, Args),
		Step::Enum::Enum::WriteLoader => WriteInitPy(&Plan.Context),
		Step::Enum::Enum::WriteVersionPin => WriteBinaryVersion(&Plan.Context),
		Step::Enum::Enum::RegisterPlugin => {
			Registration::Fn::Fn().map(|Registered| Report.Registration = Some(Registered))
		},
	};

	Outcome.map_err(|Error| Failure::Struct::Struct { Completed:std::mem::take(&mut Report.Completed), Error })?;

	Report.Completed.push(*Step);

	Ok(())
}
