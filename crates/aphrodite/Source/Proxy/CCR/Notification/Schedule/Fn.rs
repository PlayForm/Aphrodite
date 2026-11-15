//! Spawn the delivery so the create response does not wait for it.
//!
//! The task goes on `TaskTracker`, which is what makes shutdown wait for
//! in-flight notifications instead of dropping them. The success and failure
//! counters are incremented here, from the delivery's own outcome, so
//! `notify_success + notify_failure` always equals the number of deliveries
//! attempted.

use std::sync::{Arc, atomic::Ordering};

use crate::Proxy::{
	CCR::Notification::{
		Deliver::Fn::Fn as DeliverNotification,
		Event::Struct::Struct as NotificationEvent,
		Outcome::Enum::Enum as NotificationOutcome,
	},
	State::Struct::Struct as AppState,
};

/// Schedule one delivery, when a webhook is configured.
pub(crate) fn Fn(State:&Arc<AppState>, Event:NotificationEvent) {
	let Some(URL) = State.NotifyURL.clone() else {
		return;
	};

	let Tracker = State.TaskTracker.clone();
	let Client = State.Client.clone();
	let Key = State.NotifyKey.clone();
	let State = State.clone();

	Tracker.spawn(async move {
		match DeliverNotification(&Client, &URL, Key.as_deref(), &Event).await {
			NotificationOutcome::Delivered => {
				State.NotifySuccess.fetch_add(1, Ordering::Relaxed);
			},
			NotificationOutcome::Failed => {
				State.NotifyFailure.fetch_add(1, Ordering::Relaxed);
			},
		}
	});
}
