//! Plain text: the line count, and the fallback for every unclassified type.

use crate::Proxy::{Analysis::Struct::Struct as ContentAnalysis, Metadata::Field::Struct::Struct as MetadataField};

/// The one fact every text payload has.
pub(crate) fn Fn(Analysis:&ContentAnalysis<'_>) -> Vec<MetadataField> {
	vec![MetadataField::New("ln", Analysis.Lines.to_string())]
}
