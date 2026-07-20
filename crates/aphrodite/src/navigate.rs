//! Context navigation bridge — maps AphroditeState to s2-navigate's
//! ContextItems and provides the tool handler + proxy endpoint logic.
//!
//! The s2-navigate crate is agent-agnostic (no aphrodite deps). This module
//! bridges the two: it reads live state fields (markers, tool_events, files,
//! conv_index, directives) and builds ContextItems that the navigation engine
//! can index and query. Both the Hermes tool path and the proxy HTTP endpoint
//! call the same functions here.

use crate::state::AphroditeState;
use s2_navigate::{
    ContextBand, ContextItem, ContextNavigator,
    directive_item, error_item, file_item, marker_item, turn_item,
    render_navigable_index, render_cell_detail,
};

// ═════════════════════════════════════════════════════════════════════════════
// State → ContextItems bridge
// ═════════════════════════════════════════════════════════════════════════════

/// Build a ContextNavigator from live session state.
/// This is the single bridge function — everything downstream
/// (tool handler, proxy endpoint, flow assembler) calls this.
pub fn build_navigator(state: &AphroditeState) -> ContextNavigator {
    let mut items = Vec::new();

    // ── Directives ──
    for name in &state.active_directives {
        if let Some(dir) = state.directives.get(name) {
            items.push(directive_item(
                name,
                dir.content.len(),
                dir.content.len() / 4,
            ));
        }
    }

    // ── File references ──
    for (path, tool) in &state.referenced_files {
        // Look up hash from markers that match this path
        let hash = state.recent_markers.iter()
            .find(|m| m.meta.as_ref()
                .and_then(|meta| meta.get("path"))
                .map(|p| p == path)
                .unwrap_or(false))
            .map(|m| m.hash.as_str());

        items.push(file_item(
            path,
            state.turn_counter,
            tool,
            path.len() / 2, // rough token estimate
            hash,
        ));
    }

    // ── Error signatures ──
    let mut error_counts: std::collections::HashMap<u64, (usize, usize, String)> =
        std::collections::HashMap::new();
    for ev in &state.tool_events {
        if let Some(sig) = ev.error_sig {
            let entry = error_counts.entry(sig).or_insert((0, ev.turn, String::new()));
            entry.0 += 1;
            entry.1 = entry.1.min(ev.turn);
        }
    }
    for (sig, (count, turn, _text)) in error_counts {
        items.push(error_item(
            &format!("E{:04x}", sig as u16),
            turn,
            count,
            "",
        ));
    }

    // ── CCR markers ──
    for marker in &state.recent_markers {
        items.push(marker_item(
            &marker.hash,
            &marker.preview,
            marker.turn,
            marker.size,
            marker.size / 10, // compressed size estimate
        ));
    }

    // ── Conversation turns ──
    let mut sorted_turns: Vec<(usize, &(String, String, usize))> = state
        .conv_index
        .iter()
        .map(|(t, v)| (*t, v))
        .collect();
    sorted_turns.sort_by_key(|(t, _)| *t);
    for (turn, (_hash, summary, size)) in sorted_turns.iter().take(20) {
        items.push(turn_item(
            *turn,
            summary,
            *size / 4,
        ));
    }

    ContextNavigator::new(items)
}

// ═════════════════════════════════════════════════════════════════════════════
// Tool handler: aphrodite_navigate(level, cell?)
// ═════════════════════════════════════════════════════════════════════════════

/// Handle an aphrodite_navigate tool call.
///
/// Args:
///   - level: u8 (required) — S2 zoom level (0-16)
///   - cell: string (optional) — hex S2 cell ID to zoom into
///   - band: string (optional) — filter by context band name
///
/// Returns JSON with the rendered navigable index or cell detail.
pub fn handle_navigate_tool(state: &AphroditeState, args: &serde_json::Value) -> serde_json::Value {
    let level: u8 = args.get("level")
        .and_then(|v| v.as_u64())
        .map(|n| n.min(16) as u8)
        .unwrap_or(state.navigation_default_level);

    let nav = build_navigator(state);

    // If a specific cell is requested, zoom in
    if let Some(cell_hex) = args.get("cell").and_then(|v| v.as_str()) {
        let cell_id = parse_cell_hex(cell_hex);
        return match cell_id {
            Some(cid) => {
                let items = nav.navigate_to_cell(cid);
                let rendered = render_cell_detail(&items);
                serde_json::json!({
                    "level": level,
                    "cell": cell_hex,
                    "items": items.len(),
                    "content": rendered,
                })
            }
            None => serde_json::json!({
                "error": format!("invalid cell ID: {}", cell_hex),
            }),
        };
    }

    // If a band filter is requested
    if let Some(band_name) = args.get("band").and_then(|v| v.as_str()) {
        if let Some(band) = parse_band(band_name) {
            let cells = nav.cells_in_band(band, level);
            let mut items: Vec<&ContextItem> = Vec::new();
            for cid in &cells {
                items.extend(nav.navigate_to_cell(*cid));
            }
            let rendered = render_cell_detail(&items);
            return serde_json::json!({
                "level": level,
                "band": band_name,
                "cells": cells.len(),
                "items": items.len(),
                "content": rendered,
            });
        }
    }

    // Default: full navigable index at requested level
    let view = nav.context_at_level(level);
    let model_family = "code_first"; // DeepSeek-friendly
    let rendered = render_navigable_index(&view, model_family);

    serde_json::json!({
        "level": level,
        "cells": view.cell_count,
        "token_budget": view.token_budget,
        "children_available": view.children_available.len(),
        "content": rendered,
    })
}

/// Build a navigable context string suitable for injection into
/// build_turn_context (replaces or augments catalog_summary).
pub fn build_navigable_context(state: &AphroditeState) -> String {
    let nav = build_navigator(state);
    let level = state.navigation_default_level;
    let view = nav.context_at_level(level);
    render_navigable_index(&view, "code_first")
}

// ═════════════════════════════════════════════════════════════════════════════
// Helpers
// ═════════════════════════════════════════════════════════════════════════════

fn parse_cell_hex(hex: &str) -> Option<s2::cellid::CellID> {
    let hex = hex.trim_start_matches("0x").trim_start_matches("0X");
    if hex.len() > 16 {
        return None;
    }
    u64::from_str_radix(hex, 16).ok().map(s2::cellid::CellID)
}

fn parse_band(name: &str) -> Option<ContextBand> {
    match name.to_lowercase().as_str() {
        "system"       => Some(ContextBand::System),
        "directives"   => Some(ContextBand::Directives),
        "nudges"       => Some(ContextBand::Nudges),
        "state" | "plain" | "plaindata" => Some(ContextBand::PlainData),
        "files"        => Some(ContextBand::Files),
        "errors"       => Some(ContextBand::Errors),
        "history" | "turns" | "conversation" => Some(ContextBand::History),
        "markers"      => Some(ContextBand::Markers),
        "convo"        => Some(ContextBand::Conversation),
        _ => None,
    }
}
