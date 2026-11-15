//! The shape every schema record must have.
//!
//! A single malformed entry can 400 an entire request at the provider boundary,
//! so these assertions run in bulk over the whole catalogue rather than per tool -
//! a new schema is covered the moment it is added to `All`.

use crate::Schema::All;

/// Every schema must satisfy the contract the three sanitizers and the
/// `tool_describe` passthrough assume. A single malformed entry can 400 an
/// entire request at the provider boundary, so this is checked in bulk
/// rather than per tool.
#[test]
fn test_all_schemas_are_well_formed() {
	All::Fn::Fn().into_iter().for_each(|s| {
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
		top.keys().for_each(|key| {
			assert!(
				matches!(key.as_str(), "name" | "description" | "parameters"),
				"{name}: unsupported top-level schema key {key:?}"
			);
		});

		// Every parameter needs a description - an undocumented parameter is
		// exactly the hole that makes `tool_describe` useless.
		props.iter().for_each(|(pname, pschema)| {
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

			match (pschema.get("default"), pschema.get("enum").and_then(|e| e.as_array())) {
				(Some(default), Some(variants)) => {
					assert!(variants.contains(default), "{name}.{pname}: default is not one of enum");
				},
				_ => {},
			}
		});

		// Gemini rejects a `required` entry with no matching property, and
		// Hermes' sanitizer would silently drop it - catch the drift here.
		match params.get("required").and_then(|r| r.as_array()) {
			Some(required) => {
				required.iter().for_each(|entry| {
					let key = entry.as_str().expect("required entries must be strings");
					assert!(props.contains_key(key), "{name}: required lists unknown property {key:?}");
				})
			},
			None => {},
		}
	});
}

/// The deferred-catalog listing shows only the first sentence, clipped to
/// 60 chars (`tool_search._short_desc`). Anything longer is silently
/// truncated with an ellipsis, so the lead sentence has to stand alone.
#[test]
fn test_first_sentence_fits_catalog_listing() {
	All::Fn::Fn().into_iter().for_each(|s| {
		let name = s["name"].as_str().unwrap_or_default();
		let desc = s["description"].as_str().unwrap_or_default();
		let first = desc.split(['.', '!', '?', '\n']).next().unwrap_or_default();
		assert!(
			first.chars().count() <= 59,
			"{name}: first sentence is {} chars, over the 60-char catalog budget: {first:?}",
			first.chars().count()
		);
		assert!(!first.trim().is_empty(), "{name}: first sentence must not be empty");
	});
}
