//! `OnMarker`: the marker hook surface.

use crate::Scripting::{Engine::Struct::Struct, Payload::Struct::Struct as HookPayload};

impl Struct {
	/// Pre-Rust compatibility surface: run the `OnMarker` hook over the
	/// preview. Returns the transformed preview, or the input unchanged when
	/// no script modified it.
	///
	/// `Content` AND `Preview` are both seeded from the preview on purpose: the
	/// pre-Rust `OnMarker` scripts chain on `Preview`, while the generic run
	/// path chains on `Content`, so seeding both is what makes one script work
	/// under either name. `Size` is absent here - a marker already knows its
	/// size and the script does not need to be told twice.
	pub fn OnMarker(&self, hash:&str, Ct:&str, meta:&str, Preview:&str) -> String {
		let Payload = HookPayload {
			Content:Some(Preview.to_string()),
			ContentType:Some(Ct.to_string()),
			Hash:Some(hash.to_string()),
			Metadata:Some(meta.to_string()),
			Preview:Some(Preview.to_string()),
			..Default::default()
		};
		self.Run("on_marker", &Payload).unwrap_or_else(|| Preview.to_string())
	}
}
