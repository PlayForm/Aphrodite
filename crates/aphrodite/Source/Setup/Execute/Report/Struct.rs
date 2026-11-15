//! What a setup run actually completed.

use crate::Setup::{Plan::Step, Register::Outcome};

/// The steps that ran, and how registration ended.
///
/// `Completed` is the honest record: a step is pushed only after it returned
/// `Ok`, so a run that failed half way reports exactly the prefix it finished. That
/// is what makes a re-run diagnosable - the plan said what would happen, the report
/// says what did.
///
/// It is not a rollback journal. Nothing here records what a step replaced, so
/// nothing here can undo one.
pub(crate) struct Struct {
	/// The steps that returned `Ok`, in execution order.
	pub Completed:Vec<Step::Enum::Enum>,
	/// The registration outcome, when that step ran.
	pub Registration:Option<Outcome::Enum::Enum>,
}
