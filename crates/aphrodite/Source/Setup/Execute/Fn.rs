//! Executing an installation plan, step by step.

use crate::{
	Configuration::CLI::Setup::Struct::Struct as SetupArgs,
	Setup::{
		Execute::{Apply, Failure, Report},
		Plan,
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
	Plan.Steps.iter().try_fold(
		Report::Struct::Struct { Completed:Vec::new(), Registration:None },
		|mut Report, Step| Apply::Fn::Fn(&mut Report, Step, Plan, Args).map(|()| Report),
	)
}
