//! POST the event to the subscriber, once.
//!
//! A 5s timeout, because a subscriber that cannot answer in five seconds must
//! not hold a task-tracker slot open past shutdown. Every failure - transport or
//! a non-success status - is the same outcome: the event did not land.

use std::time::Duration;

use reqwest::Client as HttpClient;

use crate::Proxy::CCR::Notification::{
	Event::Struct::Struct as NotificationEvent,
	Outcome::Enum::Enum as NotificationOutcome,
};

/// Deliver one event.
pub(crate) async fn Fn(
	Client:&HttpClient,
	URL:&str,
	Key:Option<&str>,
	Event:&NotificationEvent,
) -> NotificationOutcome {
	let Request = Client.post(URL).json(Event);
	let Request = match Key {
		Some(Key) => Request.header("Authorization", format!("Bearer {Key}")),
		None => Request,
	};

	Request
		.timeout(Duration::from_secs(5))
		.send()
		.await
		.ok()
		.filter(|Response| Response.status().is_success())
		.map_or(NotificationOutcome::Failed, |_| NotificationOutcome::Delivered)
}
