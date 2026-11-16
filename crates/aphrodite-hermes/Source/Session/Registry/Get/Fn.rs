//! The get-or-create that seeds every session.

use std::{
	collections::HashMap,
	sync::{Arc, Mutex},
};

use aphrodite::State::Struct::AphroditeState;

use crate::{
	Configuration::Template,
	Session::Registry::{Const::Fallback, State::Store},
};

/// The per-session state for `sid`, created on first use.
///
/// Gets-or-creates: a new session's state is a clone of
/// `Configuration::Template::Fn::Fn` (config-seeded in production, plain
/// defaults in test builds), so every session starts from the same config
/// without re-reading `aphrodite.toml`. An EMPTY `sid` resolves to
/// `Registry::Const::Fallback` - the key for tools and tests that carry no
/// session context.
///
/// Returns an owned `Arc` clone so the registry lock is dropped before the
/// caller contends on the session's state mutex.
///
/// The lookup and the insert are two SEQUENTIAL acquisitions, never a nested
/// one: the first answers the common case, and only a miss reaches the second.
/// That is what keeps the seed - a TOML load on first call, and the config
/// watcher's thread spawn - outside the registry lock, the one lock every
/// session in the gateway shares.
pub(crate) fn Fn(sid:&str) -> Arc<Mutex<AphroditeState>> {
	let Key = match sid {
		"" => Fallback,
		sid => sid,
	};

	let Registered = {
		let Map = Store
			.get_or_init(|| Mutex::new(HashMap::new()))
			.lock()
			.unwrap_or_else(std::sync::PoisonError::into_inner);

		Map.get(Key).cloned()
	};

	match Registered {
		Some(State) => State,
		// A concurrent first call for the same key builds a second seed; the
		// loser's `Arc` is dropped by `or_insert`, which costs one clone of an
		// empty-at-seed-time state and changes no behaviour.
		None => {
			let Seed = Arc::new(Mutex::new(Template::Fn::Fn().clone()));

			let mut Map = Store
				.get_or_init(|| Mutex::new(HashMap::new()))
				.lock()
				.unwrap_or_else(std::sync::PoisonError::into_inner);

			Map.entry(Key.to_string()).or_insert(Seed).clone()
		},
	}
}
