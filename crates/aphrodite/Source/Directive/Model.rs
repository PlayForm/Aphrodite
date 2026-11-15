//! The directive data model - the loaded directive and the size caps applied
//! to directive content.
//!
//! Two caps, not one, because they bound different failures: the per-file cap
//! stops a single runaway `.md` from dominating the context, and the combined
//! cap stops several individually-acceptable directives from stacking past the
//! budget this feature is supposed to respect.

/// A loaded directive - name and content.
#[derive(Debug, Clone)]
pub struct Struct {
	pub name:String,
	pub Content:String,
}

/// Per-file cap applied when a directive `.md` is loaded from disk.
pub const MaximumDirectiveChars:usize = 2000;

/// Cap on the combined injected text across all active directives (01-F5) -
/// `MaximumDirectiveChars` alone doesn't bound this: with several directives
/// active at once, each already-capped body still stacks up in
/// `BuildDirectiveContext`'s output.
pub const MaximumCombinedChars:usize = 4000;
