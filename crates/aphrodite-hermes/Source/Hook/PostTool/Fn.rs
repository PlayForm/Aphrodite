//! The dev-only post-tool outcome line.
//!
//! Telemetry only: it writes one structured line to the session's debug file
//! and returns nothing. It must never change a hook's return value, and its
//! cost when the debug channel is off is zero - callers only invoke it after
//! `Debug::Log::Any::Fn::Fn` said yes.

use aphrodite::State::Struct::AphroditeState;

/// DEV-ONLY: post-tool outcome line built from the transform payload
/// (marker emitted with hash+type+size, auto-expanded bytes, live container
/// chunks, chain-split segments, or a passthrough reason) + result size +
/// elapsed microseconds.
///
/// An absent payload field is omitted from the line rather than written empty,
/// so the line never claims a value it lacks.
pub(crate) fn Fn(State:&AphroditeState, Session:&str, Result:&serde_json::Value, ResultBytes:usize, ElapsedUs:u64) {
	let mut Fields = vec![
		("result_bytes", ResultBytes.to_string()),
		("elapsed_us", ElapsedUs.to_string()),
		(
			"compressed",
			Result.get("compressed").and_then(|v| v.as_bool()).unwrap_or(false).to_string(),
		),
	];

	Fields.extend(
		[
			Result
				.get("reason")
				.and_then(|v| v.as_str())
				.map(|Reason| ("reason", Reason.to_string())),
			Result
				.get("hash")
				.and_then(|v| v.as_str())
				.map(|Hash| ("hash", Hash.to_string())),
			Result
				.get("type")
				.and_then(|v| v.as_str())
				.map(|Kind| ("type", Kind.to_string())),
			Result
				.get("size")
				.and_then(|v| v.as_u64())
				.map(|Size| ("size", Size.to_string())),
			Result
				.get("chunks")
				.and_then(|v| v.as_u64())
				.map(|Chunks| ("chunks", Chunks.to_string())),
			Result
				.get("segments")
				.and_then(|v| v.as_u64())
				.map(|Segments| ("segments", Segments.to_string())),
			Result.get("auto_expanded").and_then(|v| v.as_bool()).unwrap_or(false).then(|| {
				let Bytes = Result
					.get("expanded")
					.and_then(|v| v.as_str())
					.map(|Expanded| Expanded.len().to_string())
					.unwrap_or_else(|| "true".to_string());

				("auto_expanded_bytes", Bytes)
			}),
		]
		.into_iter()
		.flatten(),
	);

	crate::Debug::Log::Bridge::Fn::Fn(State, Session, "post_tool", &Fields);
}
