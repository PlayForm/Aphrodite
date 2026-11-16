//! `Compress`: the compression hook surface.

use crate::Scripting::{Engine::Struct::Struct, Payload::Struct::Struct as HookPayload};

impl Struct {
	/// Pre-Rust compatibility surface: run the `on_compress` hook over content.
	/// Returns the transformed content, or the input unchanged when no script
	/// modified it.
	///
	/// Returns the content, not an `Option`, because this is the surface the
	/// pre-Rust engine exposed and its callers pass the result straight through
	/// to storage. A `None` here would force every caller to invent the same
	/// `unwrap_or(content)` - which is exactly what "no script touched it"
	/// means on this surface.
	pub fn Compress(&self, Content:&str, Ct:&str, Size:usize) -> String {
		let Payload = HookPayload {
			Content:Some(Content.to_string()),
			Type:Some(Ct.to_string()),
			Size:Some(Size),
			..Default::default()
		};
		self.Run("on_compress", &Payload).unwrap_or_else(|| Content.to_string())
	}
}
