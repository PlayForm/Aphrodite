//! The mock upstream server and the request builders the streaming tests use.

use std::sync::Arc;

use axum::http::Method;
use bytes::Bytes;

use crate::Proxy::{Body::Capped::Struct::Struct as CappedBody, State::Struct::Struct as AppState};

pub(crate) fn HeaderValue(s:&str) -> axum::http::HeaderValue { axum::http::HeaderValue::from_str(s).unwrap() }

/// A minimal raw-TCP mock upstream: accepts one connection, drains the
/// request, and writes back a canned HTTP/1.1 response before closing -
/// this crate has no HTTP mocking dependency, and a hand-rolled listener
/// is simpler than standing up a second axum server for one test.
pub(crate) async fn Spawn(response:&'static str) -> std::net::SocketAddr {
	let Listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
	let Addr = Listener.local_addr().unwrap();
	tokio::spawn(async move {
		use tokio::io::{AsyncReadExt, AsyncWriteExt};
		let (mut Socket, _) = Listener.accept().await.unwrap();
		let mut Buffer = [0u8; 4096];
		// Drain (don't need to parse) the request up to the header terminator.
		loop {
			let n = Socket.read(&mut Buffer).await.unwrap_or(0);

			match n == 0 || Buffer[..n].windows(4).any(|w| w == b"\r\n\r\n") {
				true => {
					break;
				},
				false => {},
			}
		}
		let _ = Socket.write_all(response.as_bytes()).await;
		let _ = Socket.shutdown().await;
	});
	Addr
}

pub(crate) fn TestRequest(
	State:Arc<AppState>,
	path:&str,
	body:&'static str,
) -> (
	axum::extract::State<Arc<AppState>>,
	Method,
	axum::extract::OriginalUri,
	axum::http::HeaderMap,
	CappedBody,
) {
	(
		axum::extract::State(State),
		Method::POST,
		axum::extract::OriginalUri(format!("http://x{path}").parse().unwrap()),
		axum::http::HeaderMap::new(),
		CappedBody(Bytes::from_static(body.as_bytes())),
	)
}
