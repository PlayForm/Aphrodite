//! A file's section of a diff, summarised.

/// One file and how many hunks it carried.
///
/// `Header` is the whole `diff --git a/x b/x` line, which is what the previous reducer printed, and
/// `Hunks` is the number of `@@` headers that followed it. It is not a change summary: the hunk
/// bodies are exactly what the agent can re-read from the repository, so they are dropped.
pub(crate) struct Struct {
	pub Header:String,
	pub Hunks:usize,
}
