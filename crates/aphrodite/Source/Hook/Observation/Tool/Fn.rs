//! Observe a tool-result call.

use crate::{
	Flow,
	Hook::{
		Meta::Struct::Struct as Call,
		Observation::{Outcome::Enum::Enum as Outcome, Struct::Struct},
		Path::{Output::Fn::Fn as Output, Resolve::Fn::Fn as Resolve},
	},
};

/// Observe one tool call from the content, the tool name and Hermes' metadata.
///
/// `Okay` is fail-open: a missing `Status` means success. The written path is
/// resolved from the arguments first and the output second; the referenced path
/// is resolved from the output alone, because a reference is something the call
/// produced rather than something it was asked for - feeding arguments into it
/// would turn every `write_file` argument into a read reference.
pub(crate) fn Fn(Content:&str, ToolName:&str, Meta:&Call) -> Struct {
	let Outcome = match Meta.Status {
		Some("error") => Outcome::Failed,
		_ => Outcome::Okay,
	};
	let ErrorSig = match Outcome {
		Outcome::Okay => None,
		Outcome::Failed => Some(Flow::Signature::Error::Fn::Fn(Meta.ErrorType, Meta.ErrorMessage)),
	};

	Struct {
		Tool:ToolName.to_string(),
		Outcome,
		ArgsSig:Flow::Signature::Normalize::Fn::Fn(ToolName, Meta.ArgsJSON),
		ErrorSig,
		Bytes:Content.len(),
		WrotePath:Resolve(Content, ToolName, Meta.ArgsJSON),
		ReferencedPath:Output(Content, ToolName),
	}
}
