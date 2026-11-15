//! An error payload: the trace location, the first message and the error count.
//!
//! The trace slice walks back to a char boundary - a CJK comment before `.rs:`
//! used to panic the byte slice. `Pos` itself is a valid boundary (`.find` on
//! ASCII patterns), but the offsets around it are arbitrary byte positions and
//! can land mid-codepoint on non-ASCII lines.

use crate::Proxy::{Analysis::Struct::Struct as ContentAnalysis, Metadata::Field::Struct::Struct as MetadataField};

/// Where it happened, what it said, and how many of them there are.
pub(crate) fn Fn(Analysis:&ContentAnalysis<'_>) -> Vec<MetadataField> {
	let Content = Analysis.Content;
	let Trace = Content.lines().find_map(|Line| {
		let Trimmed = Line.trim();
		let Position = Trimmed
			.find(".rs:")
			.or_else(|| Trimmed.find(".py:"))
			.or_else(|| Trimmed.find(".go:"))?;
		let Start = (0..=Position.saturating_sub(12))
			.rev()
			.find(|Offset| Trimmed.is_char_boundary(*Offset))
			.unwrap_or(0);
		let End = (Position + 40).min(Trimmed.len());
		let End = (End..=Trimmed.len())
			.find(|Offset| Trimmed.is_char_boundary(*Offset))
			.unwrap_or(Trimmed.len());

		Some(Trimmed[Start..End].to_owned())
	});
	let Message = Content
		.lines()
		.find(|Line| Line.contains("Error:") || Line.contains("error["))
		.map(|Line| {
			let Trimmed = Line.trim();
			let Index = Trimmed.find("Error:").or_else(|| Trimmed.find("error[")).unwrap_or(0);

			Trimmed[Index..].chars().take(80).collect::<String>().replace('|', "/")
		})
		.or_else(|| {
			let FirstLine = Content.lines().next().unwrap_or("").trim();

			(!FirstLine.is_empty()).then(|| FirstLine.chars().take(80).collect::<String>().replace('|', "/"))
		});
	let ErrorCount = Content
		.lines()
		.filter(|Line| Line.contains("error") || Line.starts_with("thread '"))
		.count();

	[
		Trace
			.filter(|Trace| !Trace.is_empty())
			.map(|Trace| MetadataField::New("trace", Trace.replace('|', "/"))),
		Message.map(|Message| MetadataField::New("msg", Message)),
		(ErrorCount > 0).then(|| MetadataField::New("N_errors", ErrorCount.to_string())),
	]
	.into_iter()
	.flatten()
	.collect()
}
