//! `[previews] preview_max_chars` into the process-global preview builder.

use super::super::super::Struct::Struct;

impl Struct {
	/// Load preview settings into the process-global preview builder.
	/// `[previews] preview_max_chars` (env override:
	/// `APHRODITE_PREVIEW_MAX_CHARS`) caps the rendered preview string in
	/// chars; absent/0 = unlimited (legacy behavior). Issue #11 WS4: the key
	/// existed in the config structs but was never read anywhere - the
	/// preview builder now enforces it on every path (proxy, hooks, Hermes
	/// dylib), and this is the dylib-side wiring (the engine binary reads
	/// `MultiConfig.previews` directly in `main.rs`).
	///
	/// Zero means unlimited, not "render nothing": the builder takes
	/// `Option<u32>` and `None` is the unlimited state, so 0 must map to
	/// `None` rather than to a zero-length cap.
	pub fn ApplyPreviews(&self) {
		let Maximum = self.GetU64("APHRODITE_PREVIEW_MAX_CHARS", "previews", "preview_max_chars", 0);
		crate::Preview::State::Set::Fn(if Maximum == 0 {
			None
		} else {
			Some(Maximum.min(u32::MAX as u64) as u32)
		});
	}
}
