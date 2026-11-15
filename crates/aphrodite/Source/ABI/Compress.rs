//! `AphroditeCompress` - store content in the session's CCR store and return
//! the marker that replaces it in the caller's context.

use std::os::raw::c_char;

use headroom_core::transforms;

use super::Guard::MaximumContent;

stateful!(
	/// Compress `Content` into the session's inline store and answer with its
	/// hash, type, size, preview and ready-to-emit marker.
	///
	/// `Hint` is the caller's own type label; an empty hint or the literal
	/// `"text"` defers to the detector, which is why a host that knows better
	/// passes its own type rather than letting the classifier guess.
	///
	/// Empty content is rejected instead of stored: a zero-byte entry has a
	/// hash nobody can usefully retrieve and would occupy the store forever.
	/// Content above the 16 MB cap is rejected for the same reason a copy is
	/// refused - the caller already holds the bytes, so the engine does not
	/// need a second copy it cannot serve.
	AphroditeCompress,
	|s, Content:*const c_char, Hint:*const c_char| {
		if Content.is_empty() {
			return serde_json::json!({"error":"empty"});
		}
		if Content.len() > MaximumContent {
			return serde_json::json!({"error":"content exceeds 16MB limit"});
		}
		let Ct = transforms::content_detector::detect_content_type(&Content).content_type;
		let t = if Hint.is_empty() || Hint == "text" {
			Ct.as_str().to_string()
		} else {
			Hint.to_string()
		};
		let hash = headroom_core::ccr::compute_key(Content.as_bytes());
		s.InlineStorePut(hash.clone(), Content.to_string());
		let Preview = crate::Preview::Builder::Entry::Fn(&t, &Content);
		let Marker = crate::Marker::Format::Entry::CCRMarker(&hash, &t, Content.len(), &Preview, None, None, None);
		s.RecordMarker(crate::State::Marker::Struct::MarkerEntry {
			hash:hash.clone(),
			CCRType:t.clone(),
			Size:Content.len(),
			Preview:Preview.clone(),
			Turn:s.TurnCounter,
			Center:None,
			meta:None,
		});
		serde_json::json!({"hash":hash,"type":t,"size":Content.len(),"preview":Preview,"marker":Marker})
	}
);
