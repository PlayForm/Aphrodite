//! How precisely a build stamp identifies the build.

/// What kind of identity the stamp carries.
///
/// Two values, because there are two honest answers and one dishonest one. A
/// stamp with a revision names the source it was built from; a stamp without one
/// names only a version, and a version is shared by every build of that version.
/// The dishonest answer is to report the version in a field that reads like a
/// build identity, which is what the health probe used to do - it reported a port
/// and a version, so two different images answered identically.
pub(crate) enum Enum {
	/// The stamp names the source revision (`-dirty` when the tree differed
	/// from that revision at build time).
	Commit,
	/// No revision was stamped: this image cannot be told apart from another
	/// build of the same version, and the health payload says so. The payload
	/// string is `"version-only"` - pinned in `Struct::json`, which is what a
	/// consumer reads; the variant is only this crate's name for it.
	Version,
}
