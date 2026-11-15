//! Everything `/ccr/create` needs to store one entry, resolved from the body.
//!
//! The hash is resolved here, once: a caller-supplied key wins, and otherwise it
//! is the content's own digest. Both later stages read this value instead of
//! recomputing it - the response must name the hash the store actually recorded,
//! not the key the caller asked for.
//!
//! `Estimate` and `Notification` are `Option` because a raw-body create has
//! neither: it carries no caller-declared TTL or tags to announce, and it never
//! recorded a ratio sample. Encoding that in the command keeps the execute stage
//! free of "which kind of request was this" branching.

use crate::Proxy::CCR::Notification::Event::Struct::Struct as NotificationEvent;

/// One create, resolved.
pub(crate) struct Struct {
	/// The key the content is stored under.
	pub(crate) Hash:String,
	/// The content itself.
	pub(crate) Content:String,
	/// A compression-ratio sample to record, when this create has one.
	pub(crate) Estimate:Option<usize>,
	/// The webhook to fire once the entry lands, when this create announces one.
	pub(crate) Notification:Option<NotificationEvent>,
}
