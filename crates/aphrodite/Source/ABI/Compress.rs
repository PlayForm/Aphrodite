//! `AphroditeCompress` - store content in the session's CCR store and return
//! the marker that replaces it in the caller's context.

use std::os::raw::c_char;

use headroom_core::transforms;

use crate::ABI::{Boundary::Answer::Enum::Enum as Answer, Guard::Content::Const::MaximumContent};

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
	///
	/// Two refusals, not `Guard::Content`'s three: a stored payload's interior
	/// NUL is neutralised on the way back out by the boundary's `Transfer`, so
	/// it is not a reason to refuse the store here, and adding that check would
	/// reject content this export has always accepted.
	///
	/// # Safety
	/// `Content` and `Hint` must each be null or point to a valid,
	/// NUL-terminated C string that stays readable and unmoved for the duration
	/// of this call. The returned pointer is owned by the caller and must be
	/// released with `AphroditeFreeString`.
	AphroditeCompress,
	|s, Content:*const c_char, Hint:*const c_char| {
		crate::ABI::Guard::Reject::Fn::Fn(Content.is_empty(), || anyhow::anyhow!("empty"))?;
		crate::ABI::Guard::Reject::Fn::Fn(Content.len() > MaximumContent, || {
			anyhow::anyhow!("content exceeds 16MB limit")
		})?;

		// The hint is usable only when the caller supplied a label of its own: an
		// empty hint and the literal "text" both defer to the detector. The
		// detection therefore lives on the arm that needs it, so a caller that
		// already knows its content type never pays for a classification it
		// would discard.
		let t = Some(Hint).filter(|Hint| !Hint.is_empty() && &**Hint != "text").map_or_else(
			|| {
				transforms::content_detector::detect_content_type(&Content)
					.content_type
					.as_str()
					.to_string()
			},
			|Hint| Hint.into_owned(),
		);
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

		Ok(Answer::JSON(
			serde_json::json!({"hash":hash,"type":t,"size":Content.len(),"preview":Preview,"marker":Marker}),
		))
	}
);
