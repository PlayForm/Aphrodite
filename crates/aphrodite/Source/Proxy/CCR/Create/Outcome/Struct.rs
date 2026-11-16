//! One stored entry, as the response reports it.
//!
//! `Hash` is the hash the store actually recorded, taken from the storage
//! evidence rather than from the command, so a caller-supplied key cannot make
//! the response name a key nothing resolves.
//!
//! `CompressedSize` is the hash's own length, and that is deliberate: this
//! endpoint's documented wire contract IS the bare hash (see
//! `Test::CCR::Shape`) - no marker is rendered here, so the hash
//! is the correct subtractee, not an approximation (report 05 F5).
//!
//! `Estimate` and `Notification` are forwarded from the command: the handler
//! records the ratio sample and schedules the webhook after the write lands, and
//! does neither when the command carried neither.

use crate::Proxy::CCR::Notification::Event::Struct::Struct as NotificationEvent;

/// A stored entry.
pub(crate) struct Struct {
	/// The hash the entry is readable under.
	pub(crate) Hash:String,
	/// The content's byte length before storage.
	pub(crate) OriginalSize:usize,
	/// The bytes the response reports as the stored size - the hash length.
	pub(crate) CompressedSize:usize,
	/// The ratio sample to record, when the create carries one.
	pub(crate) Estimate:Option<usize>,
	/// The webhook to fire, when the create announces itself.
	pub(crate) Notification:Option<NotificationEvent>,
}

impl Struct {
	/// The reduction the response reports, `1.0` for empty content.
	pub(crate) fn Ratio(&self) -> f64 {
		match self.OriginalSize {
			0 => 1.0,
			_ => self.OriginalSize as f64 / self.CompressedSize.max(1) as f64,
		}
	}
}
