//! Skill registration — bundled Hermes skills shipped with the plugin.
//! These match the Python plugin's `register()` skill bundle exactly.

use serde_json::json;

/// Return all bundled skills as [(name, description)] pairs.
pub fn all_skills() -> Vec<serde_json::Value> {
    vec![
        skill("aphrodite-benchmarking", "Performance benchmarking for proxy + compression pipeline"),
        skill("aphrodite-center-testing", "Test center features end-to-end — call site audit, persistence, composition"),
        skill("aphrodite-coding-defaults", "Coding-optimized compression defaults, centers, and auto-expand"),
        skill("aphrodite-compression-architecture", "Compression architecture reference — semantic layers, token savings"),
        skill("aphrodite-context-efficiency", "Techniques for minimizing token usage when working with compressed content"),
        skill("aphrodite-dev-workflow", "End-to-end aphrodite development: cargo watch, proxy, smoke tests"),
        skill("aphrodite-hook-reference", "Complete Hermes hook API reference with parameter specs"),
        skill("aphrodite-iterate-release", "Iterative development loop: fix, bump, build, test, release"),
        skill("aphrodite-output-formatting", "LLM-native formatting rules for all output — previews, catalog, stats"),
        skill("aphrodite-presentation", "How to present features in README, docs, and user-facing content"),
        skill("aphrodite-release-workflow", "Release pipeline, version sync, budget tuning, worker config"),
        skill("aphrodite-session-patterns", "Session patterns: release pipeline, centers, worker config, metrics"),
        skill("aphrodite-tool-guide", "Full reference for CCR tools: retrieve, compress, stats, search, catalog"),
        skill("aphrodite-upgrade-breakpoints", "Cargo upgrade breakpoints checklist — axum 0.8, sha2 0.11 wildcards"),
    ]
}

fn skill(name: &str, description: &str) -> serde_json::Value {
    json!({
        "name": name,
        "description": description,
    })
}
