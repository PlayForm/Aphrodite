//! The diff reducer.
//!
//! A unified diff's value to a reader is which files changed and how much, not the hunk bodies -
//! the bodies are exactly what the agent can re-read from the repository. The reduction therefore
//! classifies each line as an event, feeds the events to a file-boundary state machine, and emits
//! one line per completed file section.
//!
//! Content with no `diff --git` header at all returns `None`: that is not a diff, and summarising it
//! as "0 files" would be a lie.

use crate::Stage2::Reduce::{
	Diff::{Event::Fn::Fn as Classify, State::Struct::Struct as State, Summary::Struct::Struct as Summary},
	Sample::Struct::Struct as Sample,
};

/// File sections the reduction shows: fifty.
const FilesShown:usize = 50;

/// Reduce diff content: file-level summary with hunk counts.
///
/// The state machine is driven in input order and flushed once at end of input, so there is exactly
/// one place that decides where a file section ends. It is not a patch reader: nothing is applied
/// and no hunk body is read.
///
/// The file sections go through the same bounded sample as every other reducer, because a
/// generated diff can carry thousands of files: the header keeps the exact file count while only
/// the first `FilesShown` sections are retained, and a section past the cap is never formatted.
pub fn ReduceDiff(Content:&str) -> Option<String> {
	let mut State = State::new();
	let mut Files = Sample::<Summary, FilesShown>::new();

	Content.lines().map(Classify).for_each(|Event| {
		State.Apply(Event).into_iter().for_each(|Summary| Files.Push(|| Summary));
	});

	State.Flush().into_iter().for_each(|Summary| Files.Push(|| Summary));

	if Files.IsEmpty() {
		return None;
	}

	let mut Parts:Vec<String> = vec![format!("[diff:{} files]", Files.Total())];

	Parts.extend(
		Files
			.IntoSample()
			.into_iter()
			.map(|File| format!("  {} ({} hunks)", File.Header, File.Hunks)),
	);

	Some(Parts.join("\n"))
}
