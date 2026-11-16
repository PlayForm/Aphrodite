//! One expansion's own state: what it has loaded, where it has been, what it may spend.

use std::collections::HashMap;

use crate::{
	Resolve::{Expansion, One},
	State::Struct::AphroditeState,
};

/// The state of exactly one expansion.
///
/// It owns the three things a walk accumulates - what it has already loaded, the path
/// it is currently on, and the byte budget a bounded expansion may spend - so none of
/// them is a parameter threaded through every call and none can outlive the retrieval
/// that created it. They used to be a `HashMap` and a `Vec` built at the entry point and
/// handed down by `&mut`, which meant a caller could seed a walk with a set it had kept
/// from an earlier retrieval, and the `Vec` was never popped: "already on the path" and
/// "seen anywhere in this tree" were the same test, so a hash first reached at the depth
/// cap stayed capped for every later branch.
///
/// The state is PRIVATE. The only authority a walk gets is the methods below - load one
/// hash, read what is cached, enter and leave the path, cache what it produced, read the
/// depth cap, spend against the budget - so a resolver cannot reach the rest of
/// `AphroditeState` even by accident. That is why the reference lives here instead of
/// every leaf taking `&mut AphroditeState`: the walk needs one store read and one
/// configured number, and that is all it can ask for.
pub(crate) struct Struct<'a> {
	/// The store and the depth cap. Private: `Load` and `Cap` are the only ways in.
	State:&'a mut AphroditeState,
	/// What this expansion has already loaded, by hash.
	Resolved:HashMap<String, Expansion::Enum::Enum>,
	/// The hashes on the current path, outermost first. A stack rather than a set: a
	/// hash leaves when its own expansion finishes, so a diamond reuses the cache while
	/// a cycle is still caught.
	Path:Vec<String>,
	/// The byte budget for a bounded expansion, or `None` for an unbounded one.
	Budget:Option<usize>,
}

impl<'a> Struct<'a> {
	/// A traversal with no budget: the explicit-resolve contract, where the whole
	/// document is returned whatever its size.
	pub(crate) fn New(State:&'a mut AphroditeState) -> Self {
		Struct { State, Resolved:HashMap::new(), Path:Vec::new(), Budget:None }
	}

	/// A traversal that abandons construction once its output would exceed `Budget`
	/// bytes: the auto-expand contract, where an over-limit result means the caller
	/// keeps its marker.
	pub(crate) fn Bounded(State:&'a mut AphroditeState, Budget:usize) -> Self {
		Struct { State, Resolved:HashMap::new(), Path:Vec::new(), Budget:Some(Budget) }
	}

	/// The configured recursion depth cap.
	pub(crate) fn Cap(&self) -> usize { self.State.RecursiveDepth }

	/// Load one hash from the store, through the same primitive every caller uses.
	pub(crate) fn Load(&mut self, HashValue:&str) -> Option<String> { One::Fn(self.State, HashValue) }

	/// What this expansion already knows about `HashValue`.
	pub(crate) fn Cached(&self, HashValue:&str) -> Option<&Expansion::Enum::Enum> { self.Resolved.get(HashValue) }

	/// Record what `HashValue` produced.
	pub(crate) fn Cache(&mut self, HashValue:&str, Value:Expansion::Enum::Enum) {
		let _ = self.Resolved.insert(HashValue.to_string(), Value);
	}

	/// Whether `HashValue` is already on the current path - the cycle test.
	pub(crate) fn OnPath(&self, HashValue:&str) -> bool { self.Path.iter().any(|OnPath| OnPath == HashValue) }

	/// Mark `HashValue` as on the path. Its matching `Leave` runs when the hash's own
	/// expansion finishes, whatever it finished with.
	pub(crate) fn Enter(&mut self, HashValue:&str) { self.Path.push(HashValue.to_string()) }

	/// Take the innermost hash off the path.
	pub(crate) fn Leave(&mut self) { let _ = self.Path.pop(); }

	/// Account for `Bytes` of constructed output, or `None` when this traversal is
	/// bounded and that output would exceed its budget.
	///
	/// The check runs BEFORE the bytes are appended, which is what makes the budget a
	/// limit on construction rather than a comparison after it: the old shape built the
	/// whole expansion and only then asked whether it fitted, so an over-limit
	/// auto-expand still paid for the entire document - and its nested expansions paid
	/// for theirs.
	pub(crate) fn Charge(&self, Bytes:usize) -> Option<()> {
		self.Budget.map_or(Some(()), |Budget| (Bytes <= Budget).then_some(()))
	}
}
