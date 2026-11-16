//! The Scripting test battery: the hook contract, chaining, error degradation, the opt-out rule
//! and the reload path.
//!
//! Each test builds an engine from in-memory scripts (`Directories` empty) so nothing reads the
//! user's real script directory; the one reload test uses a private temp dir keyed by process id
//! and removes it afterwards.
//!
//! The imports are explicit rather than `use super::*`: the parent module declares the three
//! sub-modules and nothing else, so a glob imported no type at all - `Engine` and `HookPayload`
//! were never in scope and this file did not compile under `--features scripting`.

use std::{path::PathBuf, sync::Mutex, time::SystemTime};

use crate::Scripting::{
	Engine::Struct::Struct as Engine,
	Payload::Struct::Struct as HookPayload,
	Script::Struct::Struct as Script,
};

fn Script(Source:&str) -> Script {
	Script {
		Path:PathBuf::from("test.rhai"),
		Mtime:SystemTime::UNIX_EPOCH,
		Source:Source.to_string(),
	}
}

fn Build(Scripts:Vec<Script>) -> Engine { Engine { Scripts:Mutex::new(Scripts), Directories:Vec::new() } }

// ── Contract: a script evaluates against a hook payload and returns the modified payload
// (Some) ──
#[test]
fn ScriptTransformsPayload() {
	let Subject = Build(vec![Script(
		r#"fn on_compress() {
				content + " [size=" + size.to_string() + " type=" + content_type + "]"
			}"#,
	)]);
	let Payload = HookPayload {
		Content:Some("hello".to_string()),
		Type:Some("text".to_string()),
		Size:Some(5),
		..Default::default()
	};
	let Out = Subject.Run("on_compress", &Payload);

	assert_eq!(Out.as_deref(), Some("hello [size=5 type=text]"));
}

// ── Contract: scripts chain - each script's output feeds the next script's `Content` ──
#[test]
fn ScriptOutputChainsIntoNextScript() {
	let Subject = Build(vec![
		Script("fn on_compress() { content + \"A\" }"),
		Script("fn on_compress() { content + \"B\" }"),
	]);
	let Payload = HookPayload { Content:Some("x".to_string()), ..Default::default() };

	assert_eq!(Subject.Run("on_compress", &Payload).as_deref(), Some("xAB"));
}

// ── Contract: a script that errors (throws, returns the wrong type) degrades with a warn - no
// panic - and the other scripts still apply ──
#[test]
fn ScriptErrorDegradesWithoutPanic() {
	let Subject = Build(vec![
		Script(r#"fn on_compress() { throw "boom" }"#),
		Script("fn on_compress() { 42 }"),
		Script("fn on_compress() { content + \"!\" }"),
	]);
	let Payload = HookPayload { Content:Some("x".to_string()), ..Default::default() };
	let Out = Subject.Run("on_compress", &Payload);

	assert_eq!(Out.as_deref(), Some("x!"), "bad scripts skipped, good script applied");
}

// ── Contract: a script that does not define the hook function opts out silently - no warn, no
// change ──
#[test]
fn MissingHookFnIsSkippedSilently() {
	let Subject = Build(vec![Script("fn unrelated() { 1 }")]);
	let Payload = HookPayload { Content:Some("x".to_string()), ..Default::default() };

	assert_eq!(Subject.Run("on_compress", &Payload), None);
}

// ── Contract: no scripts = pass-through unchanged (legacy surface) ──
#[test]
fn NoScriptsPassesThroughUnchanged() {
	let Subject = Build(Vec::new());

	assert_eq!(Subject.Compress("abc", "text", 3), "abc");
	assert_eq!(Subject.Marker("hash", "text", "meta", "prev"), "prev");
	assert_eq!(Subject.Retrieve("hash", "abc", "text"), "abc");
}

// ── Contract: legacy on_marker chains on the preview ──
#[test]
fn OnMarkerTransformsPreview() {
	let Subject = Build(vec![Script("fn on_marker() { preview + \"|\" + hash + \"|\" + metadata }")]);

	assert_eq!(Subject.Marker("h1", "text", "m1", "prev"), "prev|h1|m1");
}

// ── Contract: a script that fails to compile is warned and dropped at reload - the engine
// degrades to no-op rather than crashing ──
#[test]
fn BrokenScriptIsDroppedAtReload() {
	let Directory = std::env::temp_dir().join(format!("aphrodite-scripting-reload-{}", std::process::id()));
	let _ = std::fs::create_dir_all(&Directory);
	std::fs::write(Directory.join("broken.rhai"), "fn on_compress() { this is not valid rhai !!").unwrap();

	let Subject = Engine { Scripts:Mutex::new(Vec::new()), Directories:vec![Directory.clone()] };
	Subject.Reload();

	let _ = std::fs::remove_dir_all(&Directory);

	assert!(Subject.Scripts.lock().unwrap_or_else(|Error| Error.into_inner()).is_empty());

	let Payload = HookPayload { Content:Some("x".to_string()), ..Default::default() };

	assert_eq!(Subject.Run("on_compress", &Payload), None);
}

// ── Gate documentation: this module only exists when the feature is on (`Library.rs` declares
// `pub mod Scripting` behind `#[cfg(feature = "scripting")]`) ──
#[test]
fn ScriptingModuleRequiresFeature() {
	assert!(cfg!(feature = "scripting"));
}
