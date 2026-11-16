//! `aphrodite_directive` - list/swap/add/load/remove/reset active directives.
//!
//! (01-F3) delegates to the same `Directive::handle_action` the core crate's
//! `aphrodite_directive`/`aphrodite_dispatch` C ABI entry points use, so this
//! bridge exposes the identical action set and error shape - one implementation,
//! two transports.

use crate::Tool::{Arg, Dispatch::Fn::ToolHandler};

/// The `aphrodite_directive` handler.
///
/// A missing `action` defaults to `list`, which is the only safe default: every
/// other action mutates the active set.
pub(crate) fn Fn() -> (&'static str, ToolHandler) {
	("aphrodite_directive", |Args| {
		let Action = match Arg::Fn::Fn(Args, "action") {
			"" => "list",
			Action => Action,
		};
		let Name = Arg::Fn::Fn(Args, "name");

		crate::Session::With::Fn::with_shared(|State| aphrodite::Directive::Action::Fn(State, Action, Name))
	})
}
