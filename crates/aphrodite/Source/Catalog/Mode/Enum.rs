//! How much of the catalog a caller asked for.

/// The two catalog projections.
///
/// `Mode` used to be a `&str` compared with `==` in three places inside the builder,
/// so a typo in one comparison silently produced the full projection and the mode
/// itself never appeared in a type. It is parsed once at the boundary
/// ([`super::Parse::Fn::Fn`]) and matched from there, which is also what makes the
/// per-item projection a structural match rather than a repeated string test.
///
/// The wire values stay `"toc"` and anything else - the tool and the C ABI pass the
/// string through, and an unknown mode must keep meaning `Full` exactly as it did.
#[derive(Clone, Copy)]
pub enum Enum {
	/// The compact table of contents: no `turn` per item, plus recommendations.
	TOC,
	/// The full projection: every field, no recommendations.
	Full,
}
