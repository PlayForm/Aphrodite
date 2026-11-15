//! The terminal delivery policy, snapshotted once.

/// The configuration a terminal transform decides from.
///
/// It exists so selection never reads the session: every knob the plan depends
/// on is resolved into this value first, and the plan is then a pure function of
/// `(policy, input)`.
///
/// `AutoExpand` is snapshotted here rather than re-evaluated per gate because
/// its inputs (`AutoExpand`, `AutoExpandLimit`) are pure reads of the session and
/// no gate between selection and expansion mutates either one. Hoisting it is
/// therefore order-preserving - and if a future gate does start mutating those
/// fields, it has to come back through this value, which is the point.
pub struct Struct {
	/// Content within the auto-expand limit is returned inline, never marked.
	pub AutoExpand:bool,
	/// Byte threshold below which unmarked output passes through untouched.
	pub Threshold:usize,
	/// The live-container path is available.
	pub Live:bool,
	/// Chain splitting is available.
	pub ChainSplit:bool,
}
