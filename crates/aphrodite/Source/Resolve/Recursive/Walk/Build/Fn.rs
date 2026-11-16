//! The expansion itself: load, scan, resolve each occurrence, build the output.

use crate::Resolve::{
	Expansion,
	Halt,
	Nested,
	Parse::{Find, Occurrence},
	Recursive::{Recursive, Walk::Build::Writer},
	Traversal::Struct::Struct as Traversal,
};

/// Resolve one occurrence into the text that replaces it.
///
/// A hash this expansion already loaded contributes the CACHED value (F4). A
/// nested expansion whose hash the store does not hold leaves the original marker
/// (F1); one that ran out of budget abandons the whole walk, because inlining a
/// document that still carries a marker is what the budget exists to prevent.
fn Replace(Traversal:&mut Traversal, Occurrence:&Occurrence::Struct::Struct<'_>, Depth:usize) -> Nested::Enum::Enum {
	let Cached = Traversal
		.Cached(Occurrence.Hash)
		.map(Expansion::Enum::Enum::Text)
		.map(str::to_owned);

	match Cached {
		Some(Cached) => Nested::Enum::Enum::Replaced(Cached),
		None => {
			match Recursive::Fn(Traversal, Occurrence.Hash, Depth + 1) {
				Ok(Expanded) => Nested::Enum::Enum::Replaced(Expanded.Owned()),
				Err(Halt::Enum::Enum::Absent) => Nested::Enum::Enum::Unresolved,
				Err(Halt::Enum::Enum::Budget) => Nested::Enum::Enum::Abandoned,
			}
		},
	}
}

/// Expand one loaded entry, charging every segment against the traversal's budget before it
/// is appended.
///
/// Three invariants are preserved verbatim because each has a recorded failure behind it. A
/// nested hash this expansion already loaded contributes the CACHED value - skipping it left
/// the literal marker in the output (F4). An unresolved nested hash leaves the ORIGINAL
/// marker text untouched - substituting a placeholder here got permanently baked into the
/// stored entry, so a merely evicted or late nested hash could never heal (F1). And nothing is
/// written back to the store: the bytes under a hash must stay byte-identical to what the hash
/// names, so an expanded result is returned and forgotten.
///
/// The output is built in ONE pass over the scanner's occurrences, from their byte spans. The
/// previous shape folded `String::replace` over the whole document once per marker, which
/// re-read the text it had just written: a marker reached at the depth cap keeps its raw
/// marker text, and a later replacement of that same marker string then substituted inside the
/// text that had just been inserted for it.
///
/// The budget check runs before each append, so a bounded expansion stops as soon as its
/// output cannot fit instead of building the document and measuring it afterwards. A document
/// with no markers costs one load, one scan and one charge.
pub(crate) fn Fn(Traversal:&mut Traversal, Hash:&str, Depth:usize) -> Result<Expansion::Enum::Enum, Halt::Enum::Enum> {
	let Some(Content) = Traversal.Load(Hash) else {
		return Err(Halt::Enum::Enum::Absent);
	};

	// Cache what the store holds BEFORE expanding it: a reference to this hash from inside
	// its own content is a cycle, and the loaded text is the only answer that terminates.
	Traversal.Cache(Hash, Expansion::Enum::Enum::Loaded(Content.clone()));

	let Occurrences = Find::Fn(&Content);

	// Resolve every occurrence first, in document order, so the construction below is a
	// single pass with no recursion in the middle of it.
	let Replacements:Vec<Nested::Enum::Enum> = Occurrences
		.iter()
		.map(|Occurrence| Replace(Traversal, Occurrence, Depth))
		.collect();

	let Writer = Occurrences
		.iter()
		.zip(Replacements.iter())
		.try_fold(Writer::Struct::Struct { Out:String::new(), Cursor:0 }, |Writer, Pair| {
			Writer::Step::Fn::Fn(Writer, Pair, &Content, Traversal)
		})?;

	let mut Out = Writer.Out;
	let Cursor = Writer.Cursor;

	let Tail = &Content[Cursor..];

	Traversal.Charge(Out.len() + Tail.len()).ok_or(Halt::Enum::Enum::Budget)?;

	Out.push_str(Tail);

	Ok(Expansion::Enum::Enum::Complete(Out))
}
