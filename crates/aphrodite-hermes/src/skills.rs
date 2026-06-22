//! Skill registration — bundled Hermes skills shipped with the plugin.
//! Each entry must have a matching SKILL.md in the monorepo skills/ directory.

use serde_json::json;

/// Return all bundled skills as [(name, description)] pairs.
/// Sync with: ls skills/ — every entry here must have skills/<name>/SKILL.md
pub fn all_skills() -> Vec<serde_json::Value> {
    vec![
        skill("aphrodite-auto-expand-testing", "Protocol for testing auto-expand behavior — controlled by AUTO_EXPAND_LIMIT"),
        skill("aphrodite-benchmarking", "Comprehensive benchmark protocol for aphrodite proxy — compression ratios, latency, token savings"),
        skill("aphrodite-cargo-upgrade", "Cargo upgrade breakpoints for aphrodite + headroom — reqwest features, axum ConnectInfo, tokio-tungstenite"),
        skill("aphrodite-development-lessons", "Critical development pitfalls for Aphrodite — auto-expand, env_passthrough, context engine dead spots"),
        skill("aphrodite-hook-reference", "Complete Hermes hook API reference for aphrodite plugin — exact invocation signatures, parameter specs"),
        skill("aphrodite-operations", "Operational patterns for working with aphrodite — engine compression, proxy lifecycle, health checks"),
        skill("aphrodite-release-workflow", "Auto-release, version sync, pre-release verification, and release notes for Aphrodite"),
        skill("aphrodite-upgrade-breakpoints", "Cargo upgrade breakpoints for aphrodite + headroom — required by standalone plugin repo and release workflow"),
        skill("aphrodite-v0.8.6-patterns", "Historical development patterns from v0.8.5→v0.8.6 cycle — most patterns still current"),
    ]
}

fn skill(name: &str, description: &str) -> serde_json::Value {
    json!({
        "name": name,
        "description": description,
    })
}
