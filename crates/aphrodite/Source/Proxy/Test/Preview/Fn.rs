//! Proxy previews must equal the hook's: the same semantic shape has to
//! produce the same preview through both paths.

use crate::Proxy::Detect::Fn::Fn as ProxyDetectContentType;

// ── T3: build_preview ────────────────────────────────────────
// 1.5.0 (REFACTOR-PLAN §7): `proxy_build_preview` is GONE - the proxy
// routes ALL types through the single core builder
// (`crate::Preview::build_preview`, same as the Hermes hook/FFI path),
// so these pins now exercise the shared builder directly.
#[test]
fn TestBuildPreviewCodeHasCtPrefix() {
	let _G = crate::Preview::State::Guard::Fn();
	let Src = "fn add(a:i32, b:i32) -> i32 {\n    a + b\n}\n";
	let Preview = crate::Preview::Builder::Entry::Fn("code_rust", Src);
	assert!(Preview.starts_with("[code:"), "core code arm expected, got {Preview}");
}

// ── Parity (1.5.0): the proxy path IS the hook path - `SmartMarker` and
// the Hermes hook/FFI path both call `crate::Preview::build_preview`, so
// previews can never drift by construction. The pins below prove the
// classify → build round-trip still yields enriched `[type:...]` previews.
#[test]
fn TestProxyAndHookPreviewsAreIdenticalForSemanticShapes() {
	let _G = crate::Preview::State::Guard::Fn();
	let GitStatus = " M src/preview.rs\nA  src/new.rs\nD  src/old.rs\n?? tmp/x\n?? tmp/y";
	let CargoTest = "test result: ok. 220 passed; 0 failed; 1 ignored; finished in 0.31s";
	let Ripgrep = "src/a.rs:12:hit one\nsrc/a.rs:20:hit two\nsrc/b.rs:5:hit three";
	let Ls = "-rw-r--r-- 1 u g 10 x a.rs\n-rw-r--r-- 1 u g 10 x b.rs\ndrwxr-xr-x 2 u g 64 x sub";
	for Content in [GitStatus, CargoTest, Ripgrep, Ls] {
		// Classify via the proxy path, build via the single core builder -
		// the same call `SmartMarker` makes.
		let Ct = ProxyDetectContentType(Content);
		let Preview = crate::Preview::Builder::Entry::Fn(Ct, Content);
		assert!(Preview.starts_with('['), "expected enriched preview, got {Preview}");
		assert!(Preview.contains(Ct), "preview must be self-describing for ct={Ct}: {Preview}");
	}
}

#[test]
fn TestBuildPreviewErrorHasCtPrefixViaErrorLine() {
	let _G = crate::Preview::State::Guard::Fn();
	let Src = "some noise\nerror[E0308]: mismatched types\nmore noise\n";
	let Preview = crate::Preview::Builder::Entry::Fn("error", Src);
	assert!(Preview.contains("error[E0308]"));
}

#[test]
fn TestBuildPreviewDiffHasCtPrefix() {
	let _G = crate::Preview::State::Guard::Fn();
	let Src = "diff --git a/x b/x\n--- a/x\n+++ a/x\n";
	let Preview = crate::Preview::Builder::Entry::Fn("diff", Src);
	assert!(Preview.starts_with("[diff:"), "core diff arm expected, got {Preview}");
}

#[test]
fn TestBuildPreviewJSONHasCtPrefix() {
	let _G = crate::Preview::State::Guard::Fn();
	let Src = "{\"a\":1,\"b\":2}\n";
	let Preview = crate::Preview::Builder::Entry::Fn("json", Src);
	assert!(Preview.contains("keys"));
}
