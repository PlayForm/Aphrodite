//! Directive data model - the loaded directive and the size caps applied to
//! directive content.

/// A loaded directive - name and content.
#[derive(Debug, Clone)]
pub struct Directive {
	pub name:String,
	pub content:String,
}

/// Per-file cap applied when a directive `.md` is loaded from disk.
pub const MAX_DIRECTIVE_CHARS:usize = 2000;

/// Cap on the combined injected text across all active directives (01-F5) -
/// `MAX_DIRECTIVE_CHARS` alone doesn't bound this: with several directives
/// active at once, each already-capped body still stacks up in
/// `build_directive_context`'s output.
pub const MAX_COMBINED_CHARS:usize = 4000;
