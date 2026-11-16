//! The build this image actually is - and how precisely it can say so.

use crate::Health::Build::Identity;

/// The compile-time stamp of the dylib that is answering.
///
/// A version is not a build. Two images of `1.8.1` - one from a clean checkout,
/// one from a working tree with three uncommitted edits - are indistinguishable
/// by version, and a health probe that reports only a version cannot answer the
/// question an operator actually asks after a hot reload: "is the image I am
/// talking to the one I just built?".
///
/// `Commit` is therefore the identity, produced by `build.rs` from
/// `git describe --always --dirty`, so a dirty build says so instead of
/// claiming a revision it does not match. When no revision was stamped (a
/// published crate with no `.git`, a build script that could not reach git) the
/// stamp does not pretend: `identity` reports `Version`, and the health
/// payload says `"identifies": "version-only"` rather than presenting a version
/// as if it were a build identity.
pub(crate) struct Struct {
	/// The crate version (`CARGO_PKG_VERSION`).
	pub Version:&'static str,
	/// `git describe --always --dirty`, or `None` when the build had no
	/// repository to describe.
	pub Commit:Option<&'static str>,
	/// UTC ISO-8601 build timestamp.
	pub Built:Option<&'static str>,
	/// The target triple the image was compiled for.
	pub Target:Option<&'static str>,
	/// The cargo profile (`debug` / `release`).
	pub Profile:Option<&'static str>,
}

impl Struct {
	/// How precisely this stamp identifies the build.
	pub(crate) fn identity(&self) -> Identity::Enum::Enum {
		match self.Commit {
			Some(_) => Identity::Enum::Enum::Commit,
			None => Identity::Enum::Enum::Version,
		}
	}

	/// The stamp as the health payload reports it.
	///
	/// Every field is present, with `null` for the ones this build could not
	/// stamp, and `identifies` states which kind of identity the stamp is. A
	/// consumer that only knows about `version` keeps working; a consumer that
	/// wants to know whether two probes hit the same image reads `commit`.
	pub(crate) fn json(&self) -> serde_json::Value {
		let Identifies = match self.identity() {
			Identity::Enum::Enum::Commit => "commit",
			Identity::Enum::Enum::Version => "version-only",
		};

		serde_json::json!({
			"version": self.Version,
			"commit": self.Commit,
			"built": self.Built,
			"target": self.Target,
			"profile": self.Profile,
			"identifies": Identifies,
		})
	}
}
