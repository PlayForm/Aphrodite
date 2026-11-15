//! Build the announcement, or nothing.
//!
//! `None` when no webhook is configured - the subscriber is optional and a
//! create without one is a normal create, not a degraded one. The timestamp is
//! taken here rather than at delivery, so the event describes when the entry
//! landed and not when the POST got around to it.

use std::time::{SystemTime, UNIX_EPOCH};

use crate::Proxy::CCR::Notification::Event::Struct::Struct as NotificationEvent;

/// The webhook payload for a new entry, when a webhook is configured.
pub(crate) fn Fn(Hash:&str, Ttl:u64, Tags:&[String], NotifyURL:Option<&str>) -> Option<NotificationEvent> {
	NotifyURL.map(|_| {
		NotificationEvent {
			Event:"ccr_created".into(),
			hash:Hash.to_owned(),
			CreatedAt:SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_secs(),
			Ttl,
			Tags:Tags.to_vec(),
		}
	})
}
