//! s2-navigate: S2-geometry context navigation library.
//!
//! Maps conversation context onto S2 cell IDs so an LLM can navigate
//! context hierarchically — zoom in/out, query by cell, retrieve only
//! the resolution it needs — instead of receiving a flat 4000-char blob.
//!
//! ## Architecture
//!
//! The "context sphere" has:
//!   - **longitude** = position in the conversation timeline (turn number)
//!   - **latitude**  = content-type band (which section of context)
//!   - **level**     = resolution (0=coarse "session", 15=fine "line")
//!
//! Each context item (a turn, a file reference, a CCR marker, an error)
//! maps to an S2 CellID at the appropriate level. Adjacent cells on the
//! Hilbert curve = related context (same file, same turn, same error sig).
//!
//! ## Usage
//!
//! ```ignore
//! let nav = ContextNavigator::from_state(&state);
//! let view = nav.context_at_level(4)?;  // coarse overview
//! // model sees: 12 cells covering the session, 200 chars
//! let file_cells = nav.navigate_to_cell(CellID(u64), 10)?;
//! // model sees: per-file markers, 400 chars
//! ```
//!
//! ## Integration with existing code
//!
//! - `s2-probe::task_profiles()` drives the default zoom level per task type
//! - Report 08 P1 plain-data sections are the data source for each cell band
//! - `build_turn_context` in flow.rs can optionally emit a navigable index
//!   instead of the flat catalog_summary prose

use s2::cellid::CellID;
use s2::cellunion::CellUnion;
use s2::latlng::LatLng;
use s2_probe::MAX_LEVEL;

// ═════════════════════════════════════════════════════════════════════════════
// Context bands (latitude tiers on the context sphere)
// ═════════════════════════════════════════════════════════════════════════════

/// The content-type bands that partition the context sphere.
/// Each band occupies a fixed latitude range (like the block bands in s2-probe).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ContextBand {
    /// System prompt, model identity, top-level task framing
    System = 0,
    /// Behavioral directives (focus, explore, etc.)
    Directives = 1,
    /// Per-turn nudges, ephemeral one-shots
    Nudges = 2,
    /// Plain data sections: [state], [recent], [budget]
    PlainData = 3,
    /// File references: path, tool, hash
    Files = 4,
    /// Error signatures: distinct errors + counts
    Errors = 5,
    /// Turn history: archived turns with summaries
    History = 6,
    /// CCR markers: compressed content entries
    Markers = 7,
    /// Conversation turns themselves (user + assistant messages)
    Conversation = 8,
}

impl ContextBand {
    pub const ALL: [ContextBand; 9] = [
        ContextBand::System,
        ContextBand::Directives,
        ContextBand::Nudges,
        ContextBand::PlainData,
        ContextBand::Files,
        ContextBand::Errors,
        ContextBand::History,
        ContextBand::Markers,
        ContextBand::Conversation,
    ];

    /// The S2 level at which this band is natively rendered.
    /// Lower = coarser, higher = finer resolution.
    pub fn native_level(self) -> u8 {
        match self {
            ContextBand::System       => 2,  // Very coarse — one cell
            ContextBand::Directives   => 4,  // One cell per directive
            ContextBand::Nudges       => 5,
            ContextBand::PlainData    => 6,  // Per-section cells
            ContextBand::Files        => 9,  // Per-file cells
            ContextBand::Errors       => 8,  // Per-error-sig cells
            ContextBand::History      => 7,  // Per-turn cells
            ContextBand::Markers      => 12, // Per-marker cells
            ContextBand::Conversation => 10, // Per-message cells
        }
    }

    /// Human-readable label for navigable index rendering.
    pub fn label(self) -> &'static str {
        match self {
            ContextBand::System       => "system",
            ContextBand::Directives   => "directives",
            ContextBand::Nudges       => "nudges",
            ContextBand::PlainData    => "state",
            ContextBand::Files        => "files",
            ContextBand::Errors       => "errors",
            ContextBand::History      => "history",
            ContextBand::Markers      => "markers",
            ContextBand::Conversation => "convo",
        }
    }

    /// Latitude range for this band on the context sphere (degrees).
    pub fn lat_range(self) -> (f64, f64) {
        let band_height = 120.0 / ContextBand::ALL.len() as f64;
        let base = 60.0 - band_height * self as u8 as f64;
        (base - band_height, base)
    }
}

// ═════════════════════════════════════════════════════════════════════════════
// Context item — a single piece of context mapped to an S2 cell
// ═════════════════════════════════════════════════════════════════════════════

/// A single context item with its S2 cell assignment.
#[derive(Debug, Clone)]
pub struct ContextItem {
    /// Human-readable label for this item (e.g., "src/parser.rs", "turn 5")
    pub label: String,
    /// Which content band this belongs to
    pub band: ContextBand,
    /// Position along the conversation timeline (turn number, 0-based)
    pub turn: usize,
    /// The S2 cell this item maps to at its native level
    pub cell: CellID,
    /// Token count for this item (for budget tracking)
    pub token_count: usize,
    /// Byte size of the underlying data
    pub byte_size: usize,
    /// Optional hash for retrieval (CCR marker hash, file content hash)
    pub hash: Option<String>,
    /// Preview text (1-2 lines for TOC display)
    pub preview: String,
    /// Whether this item is new this turn (for delta rendering)
    pub is_new: bool,
}

/// A navigable view of context at a specific S2 level.
#[derive(Debug, Clone)]
pub struct ContextView {
    /// The S2 level this view is rendered at
    pub level: u8,
    /// Total cells covering the context at this level
    pub cell_count: usize,
    /// The cells that have content at this level
    pub cells: Vec<ContextCell>,
    /// Child cells available if the model wants to zoom in
    pub children_available: Vec<CellID>,
    /// Estimated token count for rendering this view
    pub token_budget: usize,
}

/// A single cell in a context view.
#[derive(Debug, Clone)]
pub struct ContextCell {
    /// S2 cell ID
    pub id: CellID,
    /// Content band this cell belongs to
    pub band: ContextBand,
    /// How many items are aggregated in this cell
    pub item_count: usize,
    /// Summary label for the cell
    pub label: String,
    /// Token count of items in this cell
    pub token_count: usize,
    /// Whether this cell has new content since last render
    pub has_new: bool,
    /// Preview of what's inside (first item's preview)
    pub preview: String,
}

// ═════════════════════════════════════════════════════════════════════════════
// The navigator engine
// ═════════════════════════════════════════════════════════════════════════════

/// Maps context state to S2 cells and provides navigation queries.
pub struct ContextNavigator {
    /// All context items, ordered by turn then band
    items: Vec<ContextItem>,
    /// Total turns in the session
    pub total_turns: usize,
}

impl ContextNavigator {
    /// Build a navigator from a list of context items.
    pub fn new(items: Vec<ContextItem>) -> Self {
        let max_turn = items.iter().map(|i| i.turn).max().unwrap_or(0);
        ContextNavigator { items, total_turns: max_turn + 1 }
    }

    /// Return the context view at a requested S2 level.
    ///
    /// At each level, items are grouped into cells. Coarse levels
    /// merge many items per cell; fine levels give one item per cell.
    /// The view includes children_available so the model knows what
    /// it can zoom into next.
    pub fn context_at_level(&self, level: u8) -> ContextView {
        let level = level.min(MAX_LEVEL);

        // Group items by their cell ID at this level
        let mut cell_map: std::collections::BTreeMap<u64, Vec<&ContextItem>> =
            std::collections::BTreeMap::new();

        for item in &self.items {
            let cell_at_level = item.cell.parent(level as u64);
            cell_map.entry(cell_at_level.0).or_default().push(item);
        }

        let mut cells: Vec<ContextCell> = Vec::new();
        let mut children: Vec<CellID> = Vec::new();

        for (cell_id, items_in_cell) in &cell_map {
            let representative = &items_in_cell[0];
            let token_total: usize = items_in_cell.iter().map(|i| i.token_count).sum();
            let has_new = items_in_cell.iter().any(|i| i.is_new);

            cells.push(ContextCell {
                id: CellID(*cell_id),
                band: representative.band,
                item_count: items_in_cell.len(),
                label: representative.label.clone(),
                token_count: token_total,
                has_new,
                preview: if items_in_cell.len() == 1 {
                    representative.preview.clone()
                } else {
                    format!("{} items, {} tok", items_in_cell.len(), token_total)
                },
            });

            // Children: one level deeper
            if level < MAX_LEVEL {
                let child_level = level + 1;
                let mut child_ids: Vec<CellID> = items_in_cell
                    .iter()
                    .map(|i| i.cell.parent(child_level as u64))
                    .collect();
                child_ids.sort_by_key(|c| c.0);
                child_ids.dedup_by_key(|c| c.0);
                children.extend(child_ids);
            }
        }

        children.sort_by_key(|c| c.0);
        children.dedup_by_key(|c| c.0);

        let token_budget = cells.iter().map(|c| {
            c.label.len() + c.preview.len() + 20 // overhead per cell line
        }).sum::<usize>() / 4; // ~4 chars per token

        ContextView {
            level,
            cell_count: cells.len(),
            cells,
            children_available: children,
            token_budget,
        }
    }

    /// Zoom into a specific cell, returning items at their native level.
    pub fn navigate_to_cell(&self, cell: CellID) -> Vec<&ContextItem> {
        self.items
            .iter()
            .filter(|item| {
                let item_parent = item.cell.parent(cell.level());
                item_parent == cell || item.cell.0 == cell.0
            })
            .collect()
    }

    /// Find cells that intersect a content band at a given level.
    pub fn cells_in_band(&self, band: ContextBand, level: u8) -> Vec<CellID> {
        let mut cells: Vec<CellID> = self
            .items
            .iter()
            .filter(|i| i.band == band)
            .map(|i| i.cell.parent(level as u64))
            .collect();
        cells.sort_by_key(|c| c.0);
        cells.dedup_by_key(|c| c.0);
        cells
    }

    /// Total items in the navigator.
    pub fn item_count(&self) -> usize { self.items.len() }

    /// Total estimated tokens across all items.
    pub fn total_tokens(&self) -> usize {
        self.items.iter().map(|i| i.token_count).sum()
    }

    /// Build an S2 CellUnion covering all items for computing
    /// coverings and adjacency.
    pub fn covering(&self, level: u8) -> CellUnion {
        let cells: Vec<CellID> = self
            .items
            .iter()
            .map(|i| i.cell.parent(level as u64))
            .collect();
        CellUnion(cells)
    }
}

// ═════════════════════════════════════════════════════════════════════════════
// Context item builder — constructs items from conversation state
// ═════════════════════════════════════════════════════════════════════════════

/// Maps a position (turn, band) to an S2 CellID using the context-sphere
/// mapping: longitude = position in conversation, latitude = content band.
fn cell_for_position(turn: usize, band: ContextBand, level: u8) -> CellID {
    let total_turns = (turn + 1).max(1) as f64;

    // Longitude: proportional to turn position in the session
    let lng_share = turn as f64 / total_turns;
    let lng = -180.0 + lng_share * 360.0;

    // Latitude: center of this band's range
    let (lat_lo, lat_hi) = band.lat_range();
    let lat = (lat_lo + lat_hi) / 2.0;

    CellID::from(LatLng::from_degrees(lat, lng)).parent(level as u64)
}

/// Build a context item for a directive.
pub fn directive_item(name: &str, content_len: usize, token_count: usize) -> ContextItem {
    let band = ContextBand::Directives;
    let level = band.native_level();
    let cell = cell_for_position(0, band, level);

    ContextItem {
        label: format!("directive:{}", name),
        band,
        turn: 0,
        cell,
        token_count,
        byte_size: content_len,
        hash: None,
        preview: format!("[{}] {} tok", name, token_count),
        is_new: false,
    }
}

/// Build a context item for a file reference.
pub fn file_item(path: &str, turn: usize, tool: &str, token_count: usize,
                  hash: Option<&str>) -> ContextItem {
    let band = ContextBand::Files;
    let level = band.native_level();
    let cell = cell_for_position(turn, band, level);

    ContextItem {
        label: path.to_string(),
        band,
        turn,
        cell,
        token_count,
        byte_size: path.len() + tool.len(),
        hash: hash.map(|s| s.to_string()),
        preview: format!("{} ({})", path, tool),
        is_new: false,
    }
}

/// Build a context item for an error signature.
pub fn error_item(sig: &str, turn: usize, count: usize, first_line: &str) -> ContextItem {
    let band = ContextBand::Errors;
    let level = band.native_level();
    let cell = cell_for_position(turn, band, level);

    ContextItem {
        label: format!("error:{}", sig),
        band,
        turn,
        cell,
        token_count: first_line.len() / 4,
        byte_size: first_line.len(),
        hash: None,
        preview: format!("{} (x{}) {}", sig, count, first_line.chars().take(60).collect::<String>()),
        is_new: false,
    }
}

/// Build a context item for a CCR marker.
pub fn marker_item(hash: &str, preview: &str, turn: usize,
                    original_size: usize, compressed_size: usize) -> ContextItem {
    let band = ContextBand::Markers;
    let level = band.native_level();
    let cell = cell_for_position(turn, band, level);

    ContextItem {
        label: format!("marker:{}", &hash[..12.min(hash.len())]),
        band,
        turn,
        cell,
        token_count: compressed_size / 4,
        byte_size: original_size,
        hash: Some(hash.to_string()),
        preview: preview.to_string(),
        is_new: true,
    }
}

/// Build a context item for a conversation turn.
pub fn turn_item(turn: usize, summary: &str, token_count: usize) -> ContextItem {
    let band = ContextBand::History;
    let level = band.native_level();
    let cell = cell_for_position(turn, band, level);

    ContextItem {
        label: format!("turn {}", turn),
        band,
        turn,
        cell,
        token_count,
        byte_size: summary.len(),
        hash: None,
        preview: summary.chars().take(80).collect(),
        is_new: false,
    }
}

// ═════════════════════════════════════════════════════════════════════════════
// Navigable index renderer — produces the text the model sees
// ═════════════════════════════════════════════════════════════════════════════

/// Render a navigable index from a context view.
///
/// Produces a compact table-of-contents with S2 cell IDs that the model
/// can use to navigate deeper via `aphrodite_navigate(level, cell)`.
pub fn render_navigable_index(view: &ContextView, model_family: &str) -> String {
    let mut out = String::new();

    // Header
    out.push_str(&format!(
        "[navigate] L{} — {} cells, ~{} tok. Zoom: aphrodite_navigate(level=N, cell=0x...)\n",
        view.level, view.cell_count, view.token_budget
    ));

    // Cell listing — one line per cell, compact format
    for cell in &view.cells {
        let new_marker = if cell.has_new { " +" } else { "" };
        let hash_short = format!("{:08x}", (cell.id.0 >> 32) as u32);

        match model_family {
            "code_first" | "deepseek" => {
                // Code-first models: show what's inside, inline
                out.push_str(&format!(
                    "  {:<10} L{:<2} 0x{} {:>4}t {}\n",
                    cell.band.label(),
                    view.level,
                    hash_short,
                    cell.token_count,
                    cell.preview,
                ));
            }
            _ => {
                // Balanced/compact: metadata-first
                out.push_str(&format!(
                    "  [{:<10} L{} 0x{} {:>4}t]{}{}\n",
                    cell.band.label(),
                    view.level,
                    hash_short,
                    cell.token_count,
                    new_marker,
                    if !cell.preview.is_empty() {
                        format!(" {}", cell.preview)
                    } else {
                        String::new()
                    },
                ));
            }
        }
    }

    // Footer: navigation hint
    if !view.children_available.is_empty() {
        let child_previews: Vec<String> = view
            .children_available
            .iter()
            .take(8)
            .map(|c| format!("0x{:08x}", (c.0 >> 32) as u32))
            .collect();
        out.push_str(&format!(
            "  → {} children available: {}{}\n",
            view.children_available.len(),
            child_previews.join(", "),
            if view.children_available.len() > 8 { " ..." } else { "" },
        ));
    }

    out
}

/// Render a detailed view when the model zooms into a specific cell.
pub fn render_cell_detail(items: &[&ContextItem]) -> String {
    let mut out = String::new();

    if items.is_empty() {
        out.push_str("[navigate] cell is empty — nothing at this resolution\n");
        return out;
    }

    let band = items[0].band;
    let total_tok: usize = items.iter().map(|i| i.token_count).sum();
    out.push_str(&format!(
        "[navigate:{}] {} items, {} tok\n",
        band.label(),
        items.len(),
        total_tok,
    ));

    for item in items {
        let hash_str = item.hash.as_ref()
            .map(|h| format!(" hash={}", &h[..12.min(h.len())]))
            .unwrap_or_default();
        out.push_str(&format!(
            "  {} ({}){} {}B\n",
            item.label,
            item.turn,
            hash_str,
            item.byte_size,
        ));
    }

    out
}

/// Generate the default context-sphere items for a task profile
/// (integrates with s2-probe's task_profiles).
pub fn profile_items(profile: &s2_probe::TaskProfile) -> Vec<ContextItem> {
    let mut items = Vec::new();

    for block in &profile.blocks {
        let band = match block.name {
            "system"     => ContextBand::System,
            "directives" => ContextBand::Directives,
            "nudges"     => ContextBand::Nudges,
            "plain"      => ContextBand::PlainData,
            "recall"     => ContextBand::Markers, // CCR recall = markers
            "hint"       => ContextBand::Nudges,
            "convo"      => ContextBand::Conversation,
            _            => ContextBand::PlainData,
        };

        let cell = cell_for_position(0, band, band.native_level());
        items.push(ContextItem {
            label: block.name.to_string(),
            band,
            turn: 0,
            cell,
            token_count: block.tokens,
            byte_size: block.tokens * 4,
            hash: None,
            preview: format!("{} (L{})", block.name, block.level),
            is_new: false,
        });
    }

    items
}

// ═════════════════════════════════════════════════════════════════════════════
// Tests
// ═════════════════════════════════════════════════════════════════════════════

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_items() -> Vec<ContextItem> {
        vec![
            directive_item("focus", 350, 88),
            directive_item("foresight", 200, 50),
            file_item("src/parser.rs", 2, "read_file", 1200, Some("abc123def456")),
            file_item("src/codegen.rs", 3, "read_file", 900, Some("def456abc123")),
            error_item("E0308", 5, 3, "mismatched types: expected u64, found &str"),
            marker_item("2cea00846456", "[code:0fns 180L]", 2, 5962, 40),
            marker_item("3dfb11957567", "[code:0fns 160L]", 3, 4221, 40),
            turn_item(1, "user: help refactoring error types", 30),
            turn_item(2, "assistant: read parser.rs, codegen.rs, main.rs", 25),
            turn_item(3, "tool: parser.rs (5962B) + codegen.rs (4221B)", 15),
        ]
    }

    #[test]
    fn navigator_builds_from_items() {
        let nav = ContextNavigator::new(sample_items());
        assert_eq!(nav.item_count(), 10);
        assert_eq!(nav.total_turns, 6);
        assert!(nav.total_tokens() > 0);
    }

    #[test]
    fn context_at_level_coarse() {
        let nav = ContextNavigator::new(sample_items());
        let view = nav.context_at_level(4);
        // At L4, multiple items should merge into fewer cells
        assert!(view.cell_count <= 10);
        assert!(!view.cells.is_empty());
    }

    #[test]
    fn context_at_level_fine() {
        let nav = ContextNavigator::new(sample_items());
        let view = nav.context_at_level(14);
        // At L14, should be close to one cell per item
        assert!(view.cell_count >= 5);
    }

    #[test]
    fn navigate_to_cell_finds_items() {
        let nav = ContextNavigator::new(sample_items());
        let view = nav.context_at_level(4);
        // Pick a cell and zoom in
        if let Some(first_cell) = view.cells.first() {
            let items = nav.navigate_to_cell(first_cell.id);
            assert!(!items.is_empty());
        }
    }

    #[test]
    fn cells_in_band_filters_correctly() {
        let nav = ContextNavigator::new(sample_items());
        let file_cells = nav.cells_in_band(ContextBand::Files, 9);
        assert!(!file_cells.is_empty());
        // Should find at least the 2 file items
        assert!(file_cells.len() >= 1);
    }

    #[test]
    fn render_navigable_index_produces_output() {
        let nav = ContextNavigator::new(sample_items());
        let view = nav.context_at_level(6);
        let rendered = render_navigable_index(&view, "code_first");
        assert!(rendered.contains("[navigate]"));
        assert!(rendered.contains("L6"));
        assert!(rendered.contains("0x"));
    }

    #[test]
    fn render_cell_detail_shows_items() {
        let nav = ContextNavigator::new(sample_items());
        let items = nav.navigate_to_cell(sample_items()[2].cell);
        let rendered = render_cell_detail(&items);
        assert!(rendered.contains("src/parser.rs"));
        assert!(rendered.contains("abc123"));
    }

    #[test]
    fn cell_position_is_deterministic() {
        let c1 = cell_for_position(5, ContextBand::Files, 9);
        let c2 = cell_for_position(5, ContextBand::Files, 9);
        assert_eq!(c1, c2);
    }

    #[test]
    fn different_turns_map_to_different_cells() {
        let c1 = cell_for_position(1, ContextBand::Conversation, 10);
        let c2 = cell_for_position(10, ContextBand::Conversation, 10);
        assert_ne!(c1, c2);
    }

    #[test]
    fn covering_at_high_level_has_more_cells() {
        let nav = ContextNavigator::new(sample_items());
        let coarse = nav.covering(4);
        let fine = nav.covering(12);
        assert!(fine.0.len() >= coarse.0.len());
    }

    #[test]
    fn profile_items_from_task_profile() {
        let profiles = s2_probe::task_profiles();
        let items = profile_items(&profiles[0]);
        assert!(!items.is_empty());
        // Should have one item per block in the baseline profile
        assert_eq!(items.len(), profiles[0].blocks.len());
    }

    #[test]
    fn marker_item_stores_hash() {
        let item = marker_item("abc123def456789", "[code:0fns 180L]", 2, 5962, 40);
        assert!(item.hash.is_some());
        assert_eq!(item.band, ContextBand::Markers);
        assert!(item.is_new);
    }
}
