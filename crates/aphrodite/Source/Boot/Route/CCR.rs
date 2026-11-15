//! The CCR management routes.

use std::sync::Arc;

use aphrodite::Proxy::{
	CCR::{
		Create::Handle::Fn::Fn as HandleCCRCreate,
		Delete::Fn::Fn as HandleCCRDelete,
		List::Fn::Fn as HandleCCRList,
		Reload::Fn::Fn as HandleCCRReload,
	},
	State::Struct::Struct as AppState,
};
use axum::{
	Router,
	routing::{delete, get, post},
};

/// `/ccr/*` plus `/reload` - create, list, delete and reload stored entries.
///
/// All four mutate or expose stored session content, which is why they sit on the
/// restricted router and are gated twice: the loopback + `Host` check, then the
/// management bearer token when one is configured.
pub fn Route() -> Router<Arc<AppState>> {
	Router::new()
		.route("/ccr/create", post(HandleCCRCreate))
		.route("/ccr/list", get(HandleCCRList))
		.route("/ccr/{hash}", delete(HandleCCRDelete))
		.route("/reload", post(HandleCCRReload))
}
