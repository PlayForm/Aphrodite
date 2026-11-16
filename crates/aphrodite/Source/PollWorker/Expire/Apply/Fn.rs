//! Run the retention pass over the registry.
//!
//! The thresholds live in `Expire::Decide`; this leaf only wires them to the
//! registry's one mutation that both filters and reports.

use crate::PollWorker::{
	Expire::{Decide::Fn::Fn as Decide, Report::Struct::Struct as Expiry},
	Registry::{Retain::Fn::Fn as Retain, Struct::Registry},
	Task::Struct::Struct as Record,
};

/// Apply one decision per task at `Turn`.
pub(crate) fn Fn(This:&mut Registry, Turn:usize) -> Expiry { Retain(This, |Task:&mut Record| Decide(Task, Turn)) }
