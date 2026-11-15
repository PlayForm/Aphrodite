//! Resolve the reported type and the preview source.

use headroom_core::transforms;

/// Resolve the reported `type` and the content the preview is built from.
///
/// Shared by the tool-result and terminal-output paths (F14): both resolve
/// an optional classify pair, falling back to Aphrodite's semantic detector
/// with a single terminal override, so the reported `type` AND the preview
/// both carry the high-signal shape. Detection stays in Aphrodite's layer -
/// the vendored classifier is untouched. Single type-resolution contract
/// (1.5.0 REFACTOR-PLAN §5).
///
/// `TerminalHint` enables the override: terminal output carrying an
/// `exit code:` / `Error:` line is always typed `terminal`, no matter what
/// the detector says. The tool path passes `false`, the terminal path
/// `true`.
///
/// A caller-supplied `Classify` pair wins outright and is NOT re-detected:
/// the caller already looked underneath a wrapper this layer cannot see
/// through, and second-guessing it is how the two paths would fork.
pub(crate) fn Fn<'a>(Content:&'a str, Classify:Option<(&'a str, &'a str)>, TerminalHint:bool) -> (String, &'a str) {
	match Classify {
		Some((c, t)) => (t.to_string(), c),
		None => {
			let Ct = transforms::content_detector::detect_content_type(Content).content_type;
			let t = match TerminalHint && (Content.contains("exit code:") || Content.contains("Error:")) {
				true => "terminal".to_string(),
				false => {
					// Terminal output is very often a git status / ls / test / grep
					// dump; upgrade a generic classification via Aphrodite's detector
					// so the preview is high-signal on the terminal path too.
					let Base = Ct.as_str().to_string();
					let Inp = crate::Preview::Input::Struct::Struct::new(Content)
						.unwrap_or_else(|| crate::Preview::Input::Struct::Struct::Empty(Content));
					crate::Preview::Type::Fn::Fn(&Base, &Inp).into_owned()
				},
			};
			(t, Content)
		},
	}
}
