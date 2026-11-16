//! Executing an installation plan, step by step.

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

/// Perform every step of `Plan`, recording what completed.
///
/// The plan decides *what* runs and in what order; this function only decides *how*
/// each named step is carried out. A step that fails stops the run - the remaining steps
/// depend on it, and continuing would install a layout that no later step completed.
///
/// The failure carries the steps that already ran. `Completed` is pushed only after a
/// step returns `Ok`, and on failure that prefix travels in the error instead of being
/// dropped with it, so a run that stopped at step 6 of 9 can say which five artifacts
/// are on disk. There is no rollback: nothing here records what a step replaced, so
/// nothing here can undo one.
pub(crate) fn Fn(
	Plan:&Plan::Struct::Struct,
	Args:&SetupArgs,
) -> Result<Report::Struct::Struct, Failure::Struct::Struct> {
	let mut Report = Report::Struct::Struct { Completed:Vec::new(), Registration:None };

	for Step in &Plan.Steps {
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

		match Outcome {
			Ok(()) => Report.Completed.push(*Step),
			Err(Error) => {
				return Err(Failure::Struct::Struct { Completed:Report.Completed, Error });
			},
		}
	}

	Ok(Report)
}
