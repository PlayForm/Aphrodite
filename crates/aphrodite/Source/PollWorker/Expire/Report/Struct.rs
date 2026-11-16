//! The names of the tasks an expiry call touched.

use crate::State::Identity::Command::Struct::CommandID;

/// What the expiry pass marked and what it removed.
///
/// Named rather than counted: "three tasks expired" is not actionable, while the
/// command identities let a reader correlate the pass with the nudges that
/// preceded it. `Default` is the empty report, which is what a pass over a clean
/// set produces.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct ExpirationReport {
	/// Command identities marked abandoned in this call.
	pub MarkedStale:Vec<CommandID>,
	/// Command identities dropped in this call.
	pub Removed:Vec<CommandID>,
}
