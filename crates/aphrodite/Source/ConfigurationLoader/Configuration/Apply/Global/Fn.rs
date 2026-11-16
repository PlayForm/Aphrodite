//! Publishing this document's rendering settings into the process globals.

use super::{super::super::Struct::Struct, Publish, Rendering};

impl Struct {
	/// Publish every process-global rendering setting from this document.
	///
	/// One call, once per load - not once per session. The globals are process-wide
	/// facts about one document, so publishing them from inside a per-session loop
	/// made the number of publications depend on how many sessions happened to be
	/// reachable: zero when every session lock was unavailable, N when several were,
	/// and a torn pair whenever two of those N interleaved.
	///
	/// `ApplyCompression` and `ApplyReload` write state and nothing else; this is the
	/// separate act of telling the stateless renderers what the document says.
	pub fn ApplyGlobals(&self) { Publish::Fn::Fn(&Rendering::Struct::Struct::from(self)) }
}
