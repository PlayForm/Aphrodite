//! `aphrodite_directive` - list/swap/add/load/remove/reset active directives.
//!
//! (01-F3) delegates to the same `Directive::handle_action` the core crate's
//! `aphrodite_directive`/`aphrodite_dispatch` C ABI entry points use, so this
//! bridge exposes the identical action set and error shape - one implementation,
//! two transports.

use Tool::Arg;
use Tool::Dispatch::Fn::ToolHandler;

/// The `aphrodite_directive` handler.
///
/// A missing `action` defaults to `list`, which is the only safe default: every
/// other action mutates the active set.
pub(crate) fn Fn() -> (&'static str, ToolHandler) {
	("aphrodite_directive", |args| {
		let action = Arg::Fn::Fn(args, "action");
		let action = if action.is_empty() { "list" } else { action };
		let name = Arg::Fn::Fn(args, "name");
		crate::Session::With::Fn::with_shared(|state| aphrodite::Directive::Action::Fn(state, action, name))
	})
}
