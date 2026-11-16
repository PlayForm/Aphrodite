//! `Cstr`: the caller's pointer as a bounded, borrowed view.

use std::{ffi::CStr, os::raw::c_char, ptr::NonNull};

use super::Enum::Enum as Decoded;
use crate::ABI::Guard::Content::Const::MaximumContent;

/// Decode a caller-supplied C string pointer, refusing it above the ingress cap
/// before any allocation happens.
///
/// The order is the point. A C string carries no length, so locating its
/// terminator is unavoidable - that is the caller's contract, and
/// `CStr::from_ptr` is the only way to honour it. What is avoidable is what this
/// function used to do next: `to_string_lossy().into_owned()` copied the WHOLE
/// payload before the export's 16 MB check could reject it, so an oversize
/// argument cost a full copy to be told "no". Here the length is inspected on the
/// borrowed bytes and the refusal happens before the copy - and because
/// `to_string_lossy` borrows when the bytes are valid UTF-8, an accepted argument
/// usually copies nothing either.
///
/// The lossy policy is unchanged: invalid UTF-8 still decodes with replacement
/// characters rather than being refused. A pointer-only C interface cannot bound
/// the scan itself; bounding what the scan is allowed to cost is the most this
/// signature can offer, and a length-carrying entry point would be an additive ABI
/// change rather than a repair of this one.
///
/// # Safety
/// `Ptr`, if non-null, must point to a valid, NUL-terminated C string that stays
/// readable and unmoved for the duration of this call (the `CStr::from_ptr`
/// contract). The returned `Decoded` borrows from it for as long as the caller
/// chooses, which is the second half of the obligation and the reason this
/// function is `unsafe` rather than only its dereference.
pub(crate) unsafe fn Fn<'a>(Ptr:*const c_char) -> Decoded<'a> {
	let Some(Ptr) = NonNull::new(Ptr.cast_mut()) else {
		return Decoded::Absent;
	};

	// SAFETY: `Ptr` is non-null, and per this function's safety contract it
	// points to a valid, NUL-terminated C string that remains readable and
	// unmoved for the duration of this call.
	let Raw = unsafe { CStr::from_ptr(Ptr.as_ptr()) };

	if Raw.to_bytes().len() > MaximumContent {
		return Decoded::Oversize;
	}

	Decoded::Text(Raw.to_string_lossy())
}
