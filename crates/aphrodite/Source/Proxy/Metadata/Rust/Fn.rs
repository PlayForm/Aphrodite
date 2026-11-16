//! Rust source: the `fn`/`struct`/`impl`/`trait` names a reader needs to
//! navigate the file, plus its line count.

use crate::Proxy::{Analysis::Struct::Struct as ContentAnalysis, Metadata::Field::Struct::Struct as MetadataField};

/// Language, the items, then the line count.
pub(crate) fn Fn(Analysis:&ContentAnalysis<'_>) -> Vec<MetadataField> {
	let Content = Analysis.Content();
	let Fns:Vec<&str> = Content
		.lines()
		.filter(|Line| {
			let Trimmed = Line.trim_start();

			Trimmed.starts_with("fn ") || Trimmed.starts_with("pub fn ") || Trimmed.starts_with("async fn ")
		})
		.filter_map(|Line| {
			let Trimmed = Line.trim_start();
			let AfterFn = Trimmed
				.strip_prefix("pub async fn ")
				.or_else(|| Trimmed.strip_prefix("pub fn "))
				.or_else(|| Trimmed.strip_prefix("async fn "))
				.or_else(|| Trimmed.strip_prefix("fn "))?;

			AfterFn.split(['(', ' ', '<']).next().filter(|Name| !Name.is_empty())
		})
		.collect();
	let Structs:Vec<&str> = Content
		.lines()
		.filter(|Line| {
			let Trimmed = Line.trim_start();

			Trimmed.starts_with("struct ") || Trimmed.starts_with("pub struct ")
		})
		.filter_map(|Line| {
			let Trimmed = Line.trim_start();
			let After = Trimmed
				.strip_prefix("pub struct ")
				.or_else(|| Trimmed.strip_prefix("struct "))?;

			After.split(['(', ' ', '<', '{']).next().filter(|Name| !Name.is_empty())
		})
		.collect();
	// impl blocks: `impl TypeName` or `impl Trait for TypeName` - the name a
	// reader wants is the trait, which is the first token either way.
	let Impls:Vec<&str> = Content
		.lines()
		.filter(|Line| {
			let Trimmed = Line.trim_start();

			Trimmed.starts_with("impl ") || Trimmed.starts_with("pub impl ")
		})
		.filter_map(|Line| {
			let Trimmed = Line.trim_start();
			let After = Trimmed.strip_prefix("pub impl ").or_else(|| Trimmed.strip_prefix("impl "))?;

			After.split_whitespace().next().map(|Token| Token.trim_end_matches('<'))
		})
		.collect();
	let Traits:Vec<&str> = Content
		.lines()
		.filter(|Line| {
			let Trimmed = Line.trim_start();

			Trimmed.starts_with("trait ") || Trimmed.starts_with("pub trait ")
		})
		.filter_map(|Line| {
			let Trimmed = Line.trim_start();
			let After = Trimmed.strip_prefix("pub trait ").or_else(|| Trimmed.strip_prefix("trait "))?;

			After.split([' ', '<', '{']).next().filter(|Name| !Name.is_empty())
		})
		.collect();

	[
		Some(MetadataField::New("lang", "rs")),
		(!Fns.is_empty()).then(|| MetadataField::New("fns", Fns.join(","))),
		(!Structs.is_empty()).then(|| MetadataField::New("structs", Structs.join(","))),
		(!Impls.is_empty()).then(|| MetadataField::New("impls", Impls.join(","))),
		(!Traits.is_empty()).then(|| MetadataField::New("traits", Traits.join(","))),
		Some(MetadataField::New("ln", Analysis.Lines().to_string())),
	]
	.into_iter()
	.flatten()
	.collect()
}
