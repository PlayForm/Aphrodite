//! Resolve the hash, the ratio sample and the webhook, once.
//!
//! The two body shapes differ here and only here. A JSON create takes the
//! caller's key when it supplied one, records a compression-ratio sample (the
//! content came with a declared TTL and tags, so the ratio is worth having) and
//! announces itself; a raw create is content alone and does none of those.

use headroom_core::ccr::compute_key;

use crate::Proxy::{
	CCR::{
		Create::{Command::Struct::Struct as CreateCommand, Decode::Enum::Enum as DecodedBody},
		Notification::{Build::Fn::Fn as BuildNotification, Event::Struct::Struct as NotificationEvent},
	},
	Measurement::Estimate::Fn::Fn as EstimateCompressedSize,
};

/// Build the command for a decoded body.
pub(crate) fn Fn(Decoded:DecodedBody, NotifyURL:Option<&str>) -> CreateCommand {
	match Decoded {
		DecodedBody::Json(Request) => {
			let Hash = Request.Key.unwrap_or_else(|| compute_key(Request.Content.as_bytes()));
			let Notification = BuildNotification(
				&Hash,
				Request.TtlSeconds.unwrap_or(3600),
				&Request.Tags.unwrap_or_default(),
				NotifyURL,
			);

			CreateCommand {
				Estimate:Some(EstimateCompressedSize(&Request.Content)),
				Hash,
				Content:Request.Content,
				Notification,
			}
		},
		DecodedBody::Opaque(Content) => {
			CreateCommand {
				Hash:compute_key(Content.as_bytes()),
				Content,
				Estimate:None,
				Notification:None::<NotificationEvent>,
			}
		},
	}
}
