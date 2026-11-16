//! JSON payloads and raw tool output: the top-level keys (capped at ten) and
//! an entry estimate.
//!
//! The key scan is a byte walk, not a regex, so it cannot backtrack on
//! adversarial input. It is also NOT a structural parse: it finds every
//! `"name":` run anywhere in the payload, at any depth, so a nested object's
//! keys are reported alongside the top level. Replacing it with
//! `serde_json`'s top-level key set would change what the model is told the
//! payload contains, which is a behaviour change and not a cleanup - so the walk
//! stays, and the budget is spent with `ControlFlow` instead of an error
//! sentinel that made a successful scan look like a failed traversal.

use std::ops::ControlFlow;

use crate::Proxy::{Analysis::Struct::Struct as ContentAnalysis, Metadata::Field::Struct::Struct as MetadataField};

/// How many keys the model is shown.
const KeysLimit:usize = 10;

/// The keys found, then the entry estimate.
pub(crate) fn Fn(Analysis:&ContentAnalysis<'_>) -> Vec<MetadataField> {
	let Keys = CollectKeys(Analysis.Content());
	let Entries = Analysis
		.Content()
		.lines()
		.filter(|Line| {
			let Trimmed = Line.trim();

			Trimmed.starts_with('{') || Trimmed.starts_with('"') || Trimmed.starts_with('[')
		})
		.count();

	[
		(!Keys.is_empty()).then(|| MetadataField::New("keys", Keys.join(","))),
		(Entries > 1).then(|| MetadataField::New("entries", Entries.to_string())),
	]
	.into_iter()
	.flatten()
	.collect()
}

/// Walk the payload's lines until the key budget is spent.
///
/// `ControlFlow::Break` is the budget, not a failure: the walk succeeded, it
/// simply has nothing left to report.
fn CollectKeys(Content:&str) -> Vec<String> {
	let mut Keys:Vec<String> = Vec::new();

	let _ = Content.lines().try_for_each(|Line| -> ControlFlow<()> {
		ScanLine(Line, &mut Keys);

		if Keys.len() >= KeysLimit {
			ControlFlow::Break(())
		} else {
			ControlFlow::Continue(())
		}
	});

	Keys
}

/// Find every `"key":` run on one line and append the unseen, non-private ones.
fn ScanLine(Line:&str, Keys:&mut Vec<String>) {
	let bytes = Line.as_bytes();
	let mut Index = 0usize;

	while Index + 3 < bytes.len() && Keys.len() < KeysLimit {
		let Open = match bytes[Index..].iter().position(|Byte| *Byte == b'"') {
			Some(Offset) => Index + Offset,
			None => return,
		};
		let End = bytes[Open + 1..]
			.iter()
			.position(|Byte| *Byte == b'"')
			.map(|Offset| Open + 1 + Offset)
			.unwrap_or(bytes.len());
		// `End + 2 < len` is the original's "there is a byte after the colon"
		// guard, kept so the scan's boundary behaviour is unchanged.
		let Terminated = End + 2 < bytes.len() && bytes[End + 1] == b':';

		Terminated
			.then(|| std::str::from_utf8(&bytes[Open + 1..End]).ok())
			.flatten()
			.filter(|Key| !Key.starts_with('_') && !Keys.iter().any(|Seen| Seen == Key))
			.inspect(|Key| Keys.push((*Key).to_owned()));

		Index = End + 1;
	}
}
