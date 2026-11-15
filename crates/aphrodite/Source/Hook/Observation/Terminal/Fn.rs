//! Observe a terminal call.

use crate::{
	Flow,
	Hook::Observation::{Outcome::Enum::Enum as Outcome, Struct::Struct},
};

/// Observe one terminal call from its output, its command and its exit code.
///
/// `returncode == 0` (or an absent code) is `Okay` - that is the signal the
/// error-loop breaker keys on, which is why a failing command is observed even
/// when its output is too small to compress.
///
/// Terminal failures carry no `ErrorType`, so the signature keys on the command
/// plus the first error-looking line of the output; that is what lets two
/// different failures of the same command count as different failures. A call
/// with no error-looking line falls back to its first line, so the signature is
/// never empty.
pub(crate) fn Fn(Content:&str, command:Option<&str>, Returncode:Option<i64>) -> Struct {
	let Outcome = match Returncode {
		Some(0) | None => Outcome::Okay,
		Some(_) => Outcome::Failed,
	};
	let Args = command.map(|Command| serde_json::json!({ "command": Command }));
	let ErrorSig = match Outcome {
		Outcome::Okay => None,
		Outcome::Failed => {
			let FirstError = Content
				.lines()
				.find(|Line| {
					Line.contains("error")
						|| Line.contains("Error")
						|| Line.contains("FAILED")
						|| Line.contains("panicked")
				})
				.or_else(|| Content.lines().next());

			Some(Flow::Signature::Error::Fn::ErrorSig(command, FirstError))
		},
	};

	Struct {
		Tool:"terminal".to_string(),
		Outcome,
		ArgsSig:Flow::Signature::Normalize::Fn::NormalizeArgsSig("terminal", Args.as_ref()),
		ErrorSig,
		Bytes:Content.len(),
		WrotePath:None,
		ReferencedPath:None,
	}
}
