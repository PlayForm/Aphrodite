//! Which flag governs a given session - the one decision every reader asks.
//!
//! Resolution is root-scoped and tree-wide: subagents resolve to the same root,
//! so one toggle covers a whole tree while a sibling tree stays quiet. It is
//! deliberately NOT global: a machine-wide `debug` file applies only to calls
//! that carry no session at all, so a stray global file cannot silence the
//! per-session contract.

use crate::Debug::{Chain, Flag};

/// True when the debug flag is enabled for the given session.
///
/// Resolution: the session's ROOT scoped flag (`debug.<root>` - the
/// root-most session id in the chain) wins when it exists; subagents
/// resolve to the same root, so one toggle covers the whole tree, while
/// other sessions' roots differ and stay quiet. When NO scoped flag exists
/// the session is OFF - deliberately NOT global (a machine-wide `debug`
/// file only applies to calls with no session context, e.g. a non-Hermes
/// host or tests). This keeps the toggle strictly per session tree.
///
/// It reads the filesystem on every call (through the mtime cache), so it is
/// not free - callers that need it repeatedly should hoist it, as
/// `effective_level` does.
pub(crate) fn Fn(session:&str) -> bool {
	match session.is_empty() {
		// No session context: the plain `debug` flag applies (non-Hermes
		// hosts, standalone tool paths, tests without a session).
		true => Flag::Read::Fn::Fn(&Flag::Global::Fn::Fn()),
		// Root-most first: the top-level session's scoped flag governs the
		// whole tree. Walk root -> leaf and take the first scoped flag that
		// EXISTS (its value decides; absence falls through to OFF).
		false => {
			Chain::Fn::Fn(session)
				.iter()
				.rev()
				.map(|sid| Flag::Scoped::Fn::Fn(sid))
				.find(|p| p.exists())
				.is_some_and(|p| Flag::Read::Fn::Fn(&p))
		},
	}
}
