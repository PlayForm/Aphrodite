//! A `CcrStore` whose writes always fail - the fixture for the F4 guard.

/// A `CcrStore` whose `Put` always fails - used to test report 02's F4
/// fix: a failed store write must never be followed by replacing
/// content with an unresolvable marker.
pub(crate) struct Struct;

impl headroom_core::ccr::CcrStore for Struct {
	// Upstream's `Put` returns nothing; the "did it land" answer is the
	// fork's `PutChecked`, which is what the F4 guard reads.
	fn put(&self, _Hash:&str, _Payload:&str) {}
	fn put_checked(&self, _Hash:&str, _Payload:&str) -> bool { false }
	fn get(&self, _Hash:&str) -> Option<String> { None }
	fn len(&self) -> usize { 0 }
	fn del(&self, _Hash:&str) -> bool { false }
}
