//! The directive data model.
//!
//! One item, one file, and the file is named for the kind: `Directive::Model::Struct` is the
//! type, which is the path every consumer already spells. The size caps are leaves under `Cap/`
//! instead, because they belong to the cutting rule rather than to the data model.

/// A loaded directive - name and content.
///
/// `name` is the identifier the model sees and the key `aphrodite_directive` addresses, so it is
/// DATA, not a display string: the file it came from is lowercase (`focus.md`) and the name is
/// that stem unchanged. Directive identifiers are never renamed - a caller's list, an active set
/// and a user's `directives/` directory all spell them.
///
/// It is also the entry's own copy of its map key, which is why it is documented rather than
/// removed: `Directive::Build` renders the KEY, so an entry whose copy has drifted injects its
/// body under the key that addresses it and can never appear as another directive.
#[derive(Debug, Clone)]
pub struct Struct {
	pub name:String,
	pub Content:String,
}
