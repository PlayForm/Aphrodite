//! The hook payload.

/// Hook payload handed to a script. Each field is exposed to the script as a
/// scope variable of the same name (`Content`, `ContentType`, `Size`,
/// `hash`, `Metadata`, `Preview`). `Content` is the primary transform target:
/// the value a script returns becomes the next script's `Content` (and the
/// returned payload of the hook). `Preview` mirrors the chained `Content` so
/// the pre-Rust `OnMarker` scripts (which chain on the preview) behave
/// identically.
///
/// The SCOPE names are lowercase (`hash`) because they are the names user
/// scripts already read - a `.rhai` file is a wire contract, not an internal
/// caller - while the Rust field is `Hash` like every other field here.
///
/// Every field is optional because the hook surfaces do not all carry the same
/// facts: `OnCompress` has no hash, `OnMarker` has no size. A missing field is
/// seeded as the empty string / zero in the script scope, never as a Rhai error,
/// so a script written for one hook still runs on another.
#[derive(Debug, Clone, Default)]
pub struct Struct {
	/// The content being transformed; the value the script returns replaces it.
	pub Content:Option<String>,
	/// The content type the engine classified the payload as.
	pub ContentType:Option<String>,
	/// Byte size of the original content, when the caller knows it.
	pub Size:Option<usize>,
	/// The CCR hash, on the hooks that run after a marker exists.
	pub Hash:Option<String>,
	/// The marker metadata envelope, on the marker hook.
	pub Metadata:Option<String>,
	/// The preview text, mirrored into `Content` on the marker hook.
	pub Preview:Option<String>,
}
