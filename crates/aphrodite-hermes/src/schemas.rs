//! Hermes tool schemas - JSON Schema definitions for all aphrodite tools.
//!
//! These are what the agent actually sees. Two consumers with very different
//! budgets read the same string, which drives the shape of every description
//! below:
//!
//! * The **deferred-catalog listing** (Hermes `tools/tool_search.py`,
//!   `_short_desc`) renders ONE line per tool: the first sentence, clipped to
//!   60 chars. So sentence one must be short, self-contained, and free of
//!   abbreviation periods ("e.g.") that would truncate it early.
//! * **`tool_describe`** returns the full record - but only
//!   `{name, description, parameters}`. It is a verbatim passthrough of the
//!   registered schema (`dispatch_tool_describe`), so any key we invent at
//!   the top level (`returns`, `examples`, `cost`) is silently dropped there
//!   AND forwarded into the OpenAI `function` object by
//!   `registry.get_definitions()`, where strict providers reject unknown
//!   fields. The return shape therefore lives INSIDE `description`, which is
//!   the only channel that reaches the model.
//!
//! Keyword choice is constrained by the sanitizers every schema passes
//! through: `tools/schema_sanitizer.py` (llama.cpp grammar safety),
//! `agent/gemini_schema.py` (Gemini `Schema` allowlist), and
//! `agent/anthropic_adapter.py::_normalize_tool_input_schema`. `enum`,
//! `default`, `minItems`, `minLength`, `pattern`, `items` and boolean
//! `additionalProperties` all survive that gauntlet; top-level
//! `anyOf`/`oneOf`/`allOf` do not, so mutually-exclusive arguments (hash vs
//! path) are expressed in prose instead of as a union.

use serde_json::json;

/// Parameter block for a tool that takes no arguments.
///
/// `additionalProperties: false` is deliberate: it is the only signal that
/// stops a model from inventing plausible-looking arguments for an argument-
/// less tool, and every backend either honors it or strips it harmlessly.
fn no_params() -> serde_json::Value {
	json!({"type": "object", "properties": {}, "additionalProperties": false})
}

/// Return all tool schemas as a JSON array.
pub fn all_schemas() -> Vec<serde_json::Value> {
	vec![
		schema_compress(),
		schema_retrieve(),
		schema_stats(),
		schema_files(),
		schema_diff(),
		schema_search(),
		schema_directive(),
		schema_test(),
		schema_catalog(),
		schema_reclassify(),
		schema_prefetch(),
		schema_prefetch_status(),
		schema_rebuild(),
		#[cfg(feature = "navigation")]
		schema_navigate(),
	]
}

/// Get a single tool schema by name.
pub fn get_schema(name: &str) -> Option<serde_json::Value> {
	all_schemas().into_iter().find(|s| s["name"] == name)
}

fn schema_compress() -> serde_json::Value {
	json!({
		"name": "aphrodite_compress",
		"description": "Store content in CCR and get a marker back. \
			Hashes the content, classifies it (or trusts the `type` hint), keeps it in \
			the session store, and hands back a resolvable `<<<CCR:hash|type|size>>>` \
			marker - park bulky text here instead of carrying it in context, then pull \
			it back with aphrodite_retrieve when you actually need it. Storage is \
			in-process and sub-millisecond; markers stay resolvable for the life of the \
			session but do not survive a dylib hot-reload. \
			Returns {hash, type, size, preview, marker}.",
		"parameters": {
			"type": "object",
			"properties": {
				"content": {
					"type": "string",
					"minLength": 1,
					"description": "Exact text to store. Round-trips byte-for-byte through aphrodite_retrieve."
				},
				"type": {
					"type": "string",
					"enum": ["code", "log", "diff", "error", "json", "build_output", "text"],
					"description": "Content-type hint that steers the preview and compression profile. Omit to auto-detect; \"text\" is treated as no hint."
				},
				"_ccr_center": {
					"type": "string",
					"description": "Optional label carried alongside the marker and echoed back by aphrodite_catalog - useful for tagging what a stored blob was for."
				}
			},
			"required": ["content"],
			"additionalProperties": false
		}
	})
}

fn schema_retrieve() -> serde_json::Value {
	json!({
		"name": "aphrodite_retrieve",
		"description": "Expand a CCR marker, or read a file, to full content. \
			Supply `hash` to resolve a marker recorded this session (nested markers are \
			expanded recursively), or `path` to read a file straight from disk. Give one \
			or the other - if both are present `path` wins. `path` reads are confined to \
			the current workspace and capped at 10 MiB, so paths outside it are refused. \
			`query` narrows the result to matching lines, which is the cheap way to poke \
			at a large entry without pulling all of it into context. \
			Returns {found: true, source: \"ccr\"|\"path\", hash|path, content} on success, \
			{found: false, error} when the hash is unknown or the read is refused.",
		"parameters": {
			"type": "object",
			"properties": {
				"hash": {
					"type": "string",
					"pattern": "^[0-9a-fA-F]{8,64}$",
					"description": "Full hex hash from a `<<<CCR:hash|type|size>>>` marker. Truncated hashes do not resolve - exact match only."
				},
				"query": {
					"type": "string",
					"description": "Case-insensitive substring; only lines containing it are returned."
				},
				"path": {
					"type": "string",
					"description": "Workspace-relative or absolute file path to read directly, bypassing CCR."
				}
			},
			"additionalProperties": false
		}
	})
}

fn schema_stats() -> serde_json::Value {
	json!({
		"name": "aphrodite_stats",
		"description": "Session and proxy health counters for the CCR engine. \
			One call covers both halves of the system: session state held by the dylib, \
			and a live TCP poll of the two proxy ports. Takes no arguments. \
			Returns {version, engine, inline_entries, markers, referenced_files, \
			archived_turns, turn, engine_enabled, threshold_pct, tool_threshold, \
			terminal_threshold, proxies: {token: {port, alive}, cache: {port, alive}}}. \
			`alive: false` means nothing answered on that port within 400ms - the plugin \
			still works, it just is not proxying.",
		"parameters": no_params()
	})
}

fn schema_files() -> serde_json::Value {
	json!({
		"name": "aphrodite_files",
		"description": "List file paths touched this session. \
			Populated as tool hooks observe reads, writes and prefetches, so it doubles \
			as a record of what has already been looked at. Takes no arguments. \
			Returns {total, files: [{path, tool}]} where `tool` is the tool that first \
			referenced the path.",
		"parameters": no_params()
	})
}

fn schema_diff() -> serde_json::Value {
	json!({
		"name": "aphrodite_diff",
		"description": "Per-turn history of what was compressed. \
			Walks the archived conversation index in turn order, one entry per turn that \
			produced a compression. Takes no arguments. \
			Returns {total, turns: [{turn, hash, summary, size}]}; each `hash` is \
			resolvable through aphrodite_retrieve.",
		"parameters": no_params()
	})
}

fn schema_search() -> serde_json::Value {
	json!({
		"name": "aphrodite_search",
		"description": "Find already-compressed entries by keyword or type. \
			Matches case-insensitively against each entry's preview line and its CCR \
			type - it does NOT search the full stored bodies, so treat a miss as \
			inconclusive and fall back to aphrodite_catalog. Newest first, capped at 20 \
			results. \
			Returns {query, total, results: [{hash, type, size, preview}]}.",
		"parameters": {
			"type": "object",
			"properties": {
				"query": {
					"type": "string",
					"description": "Keyword or phrase to match against preview text and CCR type."
				},
				"type": {
					"type": "string",
					"description": "Restrict results to one exact CCR type, such as source_code, terminal, build_output, diff, json, search or text."
				}
			},
			"required": ["query"],
			"additionalProperties": false
		}
	})
}

fn schema_directive() -> serde_json::Value {
	json!({
		"name": "aphrodite_directive",
		"description": "Inspect or change the active behavioral directives. \
			Directives are short instruction files injected into every turn through \
			pre_llm_call - \"focus\" for terse, minimal-tool execution, \"explore\" for \
			broad reading, and whatever else ships in directives/. \
			`list` returns {available, active, ephemeral: [{name, inline, \
			expires_after_turn}]}; `swap` replaces the whole active set with one \
			directive and returns {swapped, active}; `add` and `remove` mutate the set \
			and return {active}; `reset` clears actives, ephemerals and the manual latch, \
			handing control back to automatic selection. An unknown directive name \
			returns {error} and changes nothing.",
		"parameters": {
			"type": "object",
			"properties": {
				"action": {
					"type": "string",
					"enum": ["list", "swap", "add", "remove", "reset"],
					"default": "list",
					"description": "What to do. Defaults to `list`."
				},
				"name": {
					"type": "string",
					"description": "Directive name. Required for swap, add and remove; ignored by list and reset."
				}
			},
			"additionalProperties": false
		}
	})
}

fn schema_test() -> serde_json::Value {
	json!({
		"name": "aphrodite_test",
		"description": "Smoke-test the compress and retrieve round trip. \
			Compresses built-in samples in-process and checks each one comes back \
			byte-identical, then reports proxy health alongside. Use it to confirm the \
			engine is actually wired up before trusting a marker. \
			Returns {mode, status: \"ok\"|\"fail\", passed, total, \
			checks: [{type, hash, roundtrip}], proxies}. `status` is \"ok\" only when \
			every check round-tripped.",
		"parameters": {
			"type": "object",
			"properties": {
				"mode": {
					"type": "string",
					"enum": ["quick", "full"],
					"default": "quick",
					"description": "`quick` runs 1 sample (source code). `full` runs 3 (source code, build output with errors and warnings, JSON). Any unrecognized value behaves like `full`."
				}
			},
			"additionalProperties": false
		}
	})
}

fn schema_catalog() -> serde_json::Value {
	json!({
		"name": "aphrodite_catalog",
		"description": "List every CCR entry recorded this session. \
			The complete inventory, newest first - unlike aphrodite_search it filters \
			nothing, so this is the reliable way to find a marker you half-remember. \
			Returns {mode, total, turn, items: [...]}, where each item is \
			{hash, type, size, preview} in `toc` mode and additionally {turn, center} in \
			`full` mode.",
		"parameters": {
			"type": "object",
			"properties": {
				"mode": {
					"type": "string",
					"enum": ["full", "toc"],
					"default": "full",
					"description": "`full` includes turn and center per entry. `toc` is the compact table-of-contents form."
				}
			},
			"additionalProperties": false
		}
	})
}

fn schema_reclassify() -> serde_json::Value {
	json!({
		"name": "aphrodite_reclassify",
		"description": "Re-detect type and preview for stored entries. \
			Re-runs classification against content already in the store and rewrites the \
			catalog entry in place, which repairs entries that were typed before a \
			classifier improvement or stored with a wrong hint. Content and hashes are \
			never modified, so existing markers keep resolving. \
			Returns {status: \"ok\", reclassified: <count>}.",
		"parameters": {
			"type": "object",
			"properties": {
				"hash": {
					"type": "string",
					"pattern": "^[0-9a-fA-F]{8,64}$",
					"description": "Reclassify only this entry. Omit to reclassify every entry in the session."
				}
			},
			"additionalProperties": false
		}
	})
}

fn schema_prefetch() -> serde_json::Value {
	json!({
		"name": "aphrodite_prefetch",
		"description": "Read files now and store them as CCR markers. \
			Despite the name this is synchronous: every path is read and compressed \
			before the call returns, so each returned hash is immediately resolvable \
			through aphrodite_retrieve. Batch the files you expect to need next instead \
			of reading them one at a time - one call, one marker per file, no file \
			bodies in context. Oversized files are skipped rather than truncated. \
			Returns {total, loaded, skipped_size, missing, inline_store_bytes, \
			inline_store_byte_budget, results: [{path, status: \
			\"loaded\"|\"skipped\"|\"missing\", hash, type, size, preview, reason}]}. \
			Watch inline_store_bytes against its budget: a batch larger than the budget \
			evicts older entries, including ones from the same batch.",
		"parameters": {
			"type": "object",
			"properties": {
				"paths": {
					"type": "array",
					"items": {"type": "string"},
					"minItems": 1,
					"description": "File paths to read and store. Absolute paths are safest; relative paths resolve against the session working directory."
				}
			},
			"required": ["paths"],
			"additionalProperties": false
		}
	})
}

fn schema_prefetch_status() -> serde_json::Value {
	json!({
		"name": "aphrodite_prefetch_status",
		"description": "List prefetched files that are resolvable now. \
			Because prefetch is synchronous, `loading` and `errors` are always empty and \
			anything tracked is already `ready` - this is an inventory of file-backed \
			markers, not a progress bar. Takes no arguments. \
			Returns {loading: [], ready: [{path, hash, type, size}], errors: [], \
			total_ready}.",
		"parameters": no_params()
	})
}

fn schema_rebuild() -> serde_json::Value {
	json!({
		"name": "aphrodite_rebuild",
		"description": "Report dylib version and proxy health. \
			It does not rebuild or install anything - the loaded dylib cannot safely \
			replace itself mid-session - it reports the state an operator needs plus the \
			command to run outside the session. Takes no arguments. \
			Returns {status: \"ok\", version, proxies, hint}.",
		"parameters": no_params()
	})
}

#[cfg(feature = "navigation")]
fn schema_navigate() -> serde_json::Value {
	json!({
		"name": "aphrodite_navigate",
		"description": "Zoom through session context as an S2 cell index. \
			Maps everything compressed this session onto an S2 cell hierarchy so context \
			can be read at a chosen zoom level: low levels give a coarse overview, higher \
			levels split into finer cells. Call it without arguments for the index, then \
			pass a `cell` from that index to expand it. \
			Returns {level, cells, token_budget, children_available, content} for an \
			index view, {level, cell, items, content} for a single cell, and \
			{level, band, cells, items, content} for a band filter. An unparseable cell \
			id returns {error}.",
		"parameters": {
			"type": "object",
			"properties": {
				"level": {
					"type": "integer",
					"minimum": 0,
					"maximum": 16,
					"description": "S2 zoom level, 0 (coarsest) to 16 (finest). Defaults to the configured navigation level. Values above 16 are clamped."
				},
				"cell": {
					"type": "string",
					"description": "Hex S2 cell id taken from a previous index view; expands that cell instead of rendering the index."
				},
				"band": {
					"type": "string",
					"description": "Context band name to filter by, expanding every cell in that band at `level`."
				}
			},
			"additionalProperties": false
		}
	})
}

#[cfg(test)]
mod tests {
	use super::*;

	/// Every schema must satisfy the contract the three sanitizers and the
	/// `tool_describe` passthrough assume. A single malformed entry can 400 an
	/// entire request at the provider boundary, so this is checked in bulk
	/// rather than per tool.
	#[test]
	fn test_all_schemas_are_well_formed() {
		for s in all_schemas() {
			let name = s["name"].as_str().expect("name must be a string");
			assert!(name.starts_with("aphrodite_"), "{name}: unexpected tool name prefix");

			let desc = s["description"].as_str().unwrap_or_default();
			assert!(!desc.is_empty(), "{name}: description must not be empty");

			let params = &s["parameters"];
			assert_eq!(params["type"], "object", "{name}: parameters.type must be object");
			let props = params["properties"]
				.as_object()
				.unwrap_or_else(|| panic!("{name}: parameters.properties must be an object"));
			assert_eq!(
				params["additionalProperties"],
				serde_json::Value::Bool(false),
				"{name}: parameters must set additionalProperties: false"
			);

			// Only the three top-level keys survive `dispatch_tool_describe`;
			// anything else is dead weight that strict providers may reject
			// once `get_definitions` splices the schema into `function`.
			let top = s.as_object().expect("schema must be an object");
			for key in top.keys() {
				assert!(
					matches!(key.as_str(), "name" | "description" | "parameters"),
					"{name}: unsupported top-level schema key {key:?}"
				);
			}

			// Every parameter needs a description - an undocumented parameter is
			// exactly the hole that makes `tool_describe` useless.
			for (pname, pschema) in props {
				assert!(
					pschema.get("type").and_then(|t| t.as_str()).is_some(),
					"{name}.{pname}: parameter needs a single string `type`"
				);
				assert!(
					pschema
						.get("description")
						.and_then(|d| d.as_str())
						.is_some_and(|d| !d.is_empty()),
					"{name}.{pname}: parameter needs a non-empty description"
				);
				if let Some(default) = pschema.get("default") {
					if let Some(variants) = pschema.get("enum").and_then(|e| e.as_array()) {
						assert!(variants.contains(default), "{name}.{pname}: default is not one of enum");
					}
				}
			}

			// Gemini rejects a `required` entry with no matching property, and
			// Hermes' sanitizer would silently drop it - catch the drift here.
			if let Some(required) = params.get("required").and_then(|r| r.as_array()) {
				for entry in required {
					let key = entry.as_str().expect("required entries must be strings");
					assert!(props.contains_key(key), "{name}: required lists unknown property {key:?}");
				}
			}
		}
	}

	/// The deferred-catalog listing shows only the first sentence, clipped to
	/// 60 chars (`tool_search._short_desc`). Anything longer is silently
	/// truncated with an ellipsis, so the lead sentence has to stand alone.
	#[test]
	fn test_first_sentence_fits_catalog_listing() {
		for s in all_schemas() {
			let name = s["name"].as_str().unwrap_or_default();
			let desc = s["description"].as_str().unwrap_or_default();
			let first = desc.split(['.', '!', '?', '\n']).next().unwrap_or_default();
			assert!(
				first.chars().count() <= 59,
				"{name}: first sentence is {} chars, over the 60-char catalog budget: {first:?}",
				first.chars().count()
			);
			assert!(!first.trim().is_empty(), "{name}: first sentence must not be empty");
		}
	}

	/// Every documented tool must resolve by name, and unknown names must not.
	#[test]
	fn test_get_schema_round_trip() {
		for s in all_schemas() {
			let name = s["name"].as_str().unwrap().to_string();
			assert_eq!(get_schema(&name).unwrap(), s, "{name}: get_schema returned a different record");
		}
		assert!(get_schema("aphrodite_not_a_tool").is_none());
	}

	/// The schema list and the dispatch registry must not drift apart: a
	/// schema with no handler is an advertised tool that always errors.
	#[test]
	fn test_every_schema_has_a_handler() {
		for s in all_schemas() {
			let name = s["name"].as_str().unwrap();
			let result = crate::tools::dispatch(name, "{}");
			let err = result.get("error").and_then(|e| e.as_str()).unwrap_or_default();
			assert!(
				!err.starts_with("unknown tool"),
				"{name}: schema advertises a tool with no handler"
			);
		}
	}
}
