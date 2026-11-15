//! The directive data model.
//!
//! One item, one file, and the file is named for the kind: `Directive::Model::Struct` is
//! the type, which is the path every consumer already spells. The size caps are leaves
//! under `Cap/` instead, because they belong to the cutting rule rather than to the data
//! model.

/// A loaded directive - name and content.
///
/// `name` is the identifier the model sees and the key `aphrodite_directive` addresses,
/// so it is data, not a display string: the file it came from is lowercase (`focus.md`)
/// and the name is that stem unchanged.
#[derive(Debug, Clone)]
pub struct Struct {
	pub name:String,
	pub Content:String,
}
