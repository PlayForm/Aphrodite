//! What a script sees, as a Rhai scope.

use rhai::Scope;

use crate::Scripting::Payload::Struct::Struct as HookPayload;

/// Seed the scope a hook function reads.
///
/// This is the DECODE step of the payload contract, and it lives in one leaf because
/// the seven names below are the whole of what a script can see. Spelled inline in the
/// call path they were seven statements a reader had to diff against the payload struct
/// to trust.
///
/// Every missing field is seeded as the empty string or zero, never as a Rhai error: the
/// hook surfaces do not all carry the same facts (`Compress` has no hash, `Marker`
/// has no size), and a script written for one surface must still run on another.
///
/// `preview` mirrors the CURRENT content rather than the payload's original, which is
/// what makes the pre-Rust `on_marker` scripts - which chain on the preview - behave
/// identically to the ones that chain on the content.
pub(crate) fn Fn(Payload:&HookPayload, Hook:&str, Content:&str) -> Scope<'static> {
	let mut Scope = Scope::new();

	// `push` only fails on a duplicate name - the names below are distinct, so the
	// Result is deliberately ignored.
	let _ = Scope.push("content", Content.to_string());
	let _ = Scope.push("hook", Hook.to_string());
	let _ = Scope.push("content_type", Payload.Type.clone().unwrap_or_default());
	let _ = Scope.push("size", Payload.Size.map(|Size| Size as i64).unwrap_or(0));
	let _ = Scope.push("hash", Payload.Hash.clone().unwrap_or_default());
	let _ = Scope.push("metadata", Payload.Metadata.clone().unwrap_or_default());
	let _ = Scope.push("preview", Content.to_string());

	Scope
}
