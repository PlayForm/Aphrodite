//! The tool-result delivery policy, snapshotted once.

/// The configuration a tool-result transform decides from.
///
/// It exists so selection never reads the session: every knob the plan depends
/// on is resolved into this value first, and the plan is then a pure function of
/// `(policy, input)`.
///
/// `AutoExpand` is snapshotted here rather than re-evaluated per gate. That is
/// order-preserving ONLY because its inputs (`AutoExpand`, `AutoExpandLimit`) are
/// pure reads of the session and nothing between selection and expansion mutates
/// either one - the store, the marker ring, classification and preview rendering
/// all leave them alone. A future gate that touches those fields has to come back
/// through this value; hoisting the predicate without that check would change
/// when it is decided.
pub struct Struct {
	/// Content within the auto-expand limit is returned inline, never marked.
	pub AutoExpand:bool,
	/// Byte threshold below which content passes through untouched.
	pub Threshold:usize,
	/// The live-container path is available.
	pub Live:bool,
}
