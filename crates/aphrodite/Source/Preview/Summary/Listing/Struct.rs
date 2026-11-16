//! The directory-listing tally.

use std::collections::HashMap;

use crate::Preview::Parse::Listing::{Entry::Struct::Struct as Entry, Kind::Enum::Enum as Kind};

/// Characters an extension key may occupy, including its leading dot: eight.
///
/// The key is bounded because it is a map key, not because it is displayed - an
/// unbounded key would let a pathological listing grow the tally without limit. It is
/// not a display cap: the renderer prints the key it finds.
const Extension:usize = 8;

/// A parsed listing: how many files, how many directories, and the extension tally.
///
/// The tally is kept whole and ordered on demand, so the summary does not have to
/// re-sort on every push. It is not a filesystem view - there are no sizes, times or
/// permissions, because the preview never claimed to carry them.
pub(crate) struct Struct {
	Files:usize,
	Directories:usize,
	Extensions:HashMap<String, usize>,
}

impl Struct {
	pub(crate) fn new() -> Self { Self { Files:0, Directories:0, Extensions:HashMap::new() } }

	/// Account for one entry.
	///
	/// A directory contributes only to the directory count. A file contributes to the
	/// file count and, when its basename has an extension that is not the whole name
	/// and not a leading dot, to the extension tally. It is not a MIME guess: the text
	/// after the last dot is the key, whatever it says.
	pub(crate) fn Push(&mut self, Entry:Entry) {
		match Entry.Kind {
			Kind::Directory => self.Directories += 1,
			Kind::File => {
				self.Files += 1;

				let Base = Entry.Name.rsplit('/').next().unwrap_or(&Entry.Name);

				Base.rfind('.')
					.filter(|Dot| *Dot > 0 && *Dot < Base.len() - 1)
					.map(|Dot| Base[Dot..].chars().take(Extension).collect::<String>())
					.into_iter()
					.for_each(|Key| *self.Extensions.entry(Key).or_insert(0) += 1);
			},
		}
	}

	/// The most common extensions, most frequent first, ties broken by name.
	///
	/// Ordering happens here rather than in the renderer because it is derived from the
	/// tally - re-deriving it at render time is how the two drift apart. It is not a
	/// stable ranking: two extensions with the same count are ordered by name, which is
	/// the tie-break the previous reader used.
	pub(crate) fn Extensions(&self, Shown:usize) -> Vec<(&str, usize)> {
		let mut Sorted:Vec<(&str, usize)> = self.Extensions.iter().map(|(Key, Count)| (Key.as_str(), *Count)).collect();

		Sorted.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(b.0)));

		Sorted.into_iter().take(Shown).collect()
	}

	pub(crate) fn Files(&self) -> usize { self.Files }
	pub(crate) fn Directories(&self) -> usize { self.Directories }

	/// True when nothing was recognised - the arm then renders a bare line count.
	pub(crate) fn IsEmpty(&self) -> bool { self.Files == 0 && self.Directories == 0 }
}
