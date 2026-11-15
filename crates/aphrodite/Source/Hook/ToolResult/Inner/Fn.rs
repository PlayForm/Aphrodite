//! The tool-result gate sequence.

use super::super::{Classify, Record};
use crate::{
	Hook::{
		AutoExpand::{Applies, Content},
		Live::Container,
		Meta::Struct::Struct as ToolCallMeta,
		Path::Fn::Fn as ExtractFilePath,
		Tool::Const::EssentialTools,
	},
	Marker::Format::Entry::CCRMarker,
	State::{Marker::Struct::MarkerEntry, Struct::AphroditeState},
};

/// The tool-result pipeline: telemetry, file tracking, bypass gates, the live
/// container, then store + marker + the auto-expand override.
///
/// Gate ORDER is the contract, not an implementation detail. Telemetry and
/// file tracking run before every early return (a bypassed tool still happened
/// and still referenced a file); auto-expand is consulted before the threshold
/// gate (it must be able to store below-threshold content, or a marker could
/// leak); the live container runs after the threshold gate and never under
/// auto-expand. Reordering any of these silently changes what the model sees.
pub(crate) fn Fn(
	State:&mut AphroditeState,
	Content:&str,
	ToolName:&str,
	Classify:Option<(&str, &str)>,
	meta:&ToolCallMeta,
) -> serde_json::Value {
	// Record the telemetry event first (P2/T6): it must happen even when the
	// content is empty or the tool is essential/self/below-threshold - the
	// phase detector and error-loop breaker care about ALL calls, not only the
	// ones that produced a compressible marker. Skip only genuinely empty
	// content (no signal at all).
	match !Content.is_empty() {
		true => {
			Record::Fn::Fn(State, ToolName, Content, meta);
		},
		false => {},
	}
	match Content.is_empty() {
		true => {
			return serde_json::json!({"status": "ok", "compressed": false, "reason": "empty"});
		},
		false => {},
	}

	// Track file references BEFORE the essential-tool early return (F10):
	// `read_file` is both listed in `EssentialTools` (never compressed - the
	// agent needs the raw content) and in `state.file_tools` (tracked for
	// `aphrodite_files`/the catalog's `ReferencedFiles` stat). Tracking used
	// to run after the essential-tool check returned early, so the single
	// most common file-reference source - reading a file - was silently
	// never recorded. Skipping compression and recording a reference are
	// independent decisions; do both regardless of which tools land in
	// which list.
	match State.FileTools.iter().any(|t| t == ToolName) {
		true => {
			match ExtractFilePath(Content, ToolName) {
				Some(path) => {
					State.RecordFile(path, ToolName.to_string());
				},
				None => {},
			}
		},
		false => {},
	}

	// Skip essential tools
	match EssentialTools.contains(&ToolName) {
		true => {
			return serde_json::json!({"status": "ok", "compressed": false, "reason": "essential_tool"});
		},
		false => {},
	}

	// Skip aphrodite's own tools (and headroom helpers): their output is already
	// compact metadata, and compressing aphrodite_retrieve would replace the
	// resolved content with another CCR marker - an infinite preview loop.
	match ToolName.starts_with("aphrodite_") || ToolName.starts_with("headroom") {
		true => {
			return serde_json::json!({"status": "ok", "compressed": false, "reason": "self_tool"});
		},
		false => {},
	}

	// Skip below threshold (0 = always compress). Auto-expand bypasses this
	// gate: under auto-expand every non-essential tool result within the
	// limit goes through the store+expand path so no marker can leak.
	match !Applies::Fn::Fn(State, Content) && State.ToolThreshold > 0 && Content.len() < State.ToolThreshold {
		true => {
			return serde_json::json!({"status": "ok", "compressed": false, "reason": "below_threshold"});
		},
		false => {},
	}

	// ── Live container (pre-Rust port of _hooks/live.py) ──
	// Enabled AND above the compress threshold → store the content chunked
	// with a metadata envelope and emit the INSTANT marker with a progress
	// annotation; the retrieve path serves the chunks progressively. When
	// auto-expand applies it wins (it guarantees no marker reaches the
	// model, so the container path is skipped and the expanded content is
	// returned inline exactly as today).
	match State.LiveContainer && !Applies::Fn::Fn(State, Content) {
		true => {
			let (TypeStr, _ClassifyContent) = Classify::Fn::Fn(Content, Classify, false);
			return Container::Fn::Fn(State, Content, &TypeStr);
		},
		false => {},
	}

	let (TypeStr, ClassifyContent) = Classify::Fn::Fn(Content, Classify, false);
	let hash = headroom_core::ccr::compute_key(Content.as_bytes());

	State.InlineStorePut(hash.clone(), Content.to_string());

	let Preview = crate::Preview::Builder::Entry::Fn(&TypeStr, ClassifyContent);
	let Marker = CCRMarker(&hash, &TypeStr, Content.len(), &Preview, None, None, None);

	State.RecordMarker(MarkerEntry {
		hash:hash.clone(),
		CCRType:TypeStr.clone(),
		Size:Content.len(),
		Preview:Preview.clone(),
		Turn:State.TurnCounter,
		Center:None,
		meta:None,
	});

	// Auto-expand (pre-Rust port): store + record exactly as above, but
	// return the RESOLVED content inline instead of the marker so no
	// `<<<CCR:...>>>` reaches the model. Over the limit or disabled, the
	// payload below is byte-identical to the pre-auto-expand shape.
	let Expanded = match Applies::Fn::Fn(State, Content) {
		true => Some(Content::Fn::Fn(State, &hash, Content)),
		false => None,
	};
	// DEV-ONLY debug metrics (crate::debug): count the compress decision.
	crate::Debug::Count::Bump::Fn::Fn(crate::Debug::Count::Enum::Enum::Compressions);
	let mut Payload = serde_json::json!({
		"status": "ok",
		"compressed": true,
		"hash": hash,
		"type": TypeStr,
		"size": Content.len(),
		"preview": Preview,
		"marker": Marker,
	});
	match Expanded {
		Some(Expanded) => {
			Payload["auto_expanded"] = serde_json::Value::Bool(true);
			Payload["expanded"] = serde_json::Value::String(Expanded);
			crate::Debug::Count::Bump::Fn::Fn(crate::Debug::Count::Enum::Enum::AutoExpansions);
		},
		None => {},
	}
	Payload
}
