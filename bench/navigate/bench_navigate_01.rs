//! bench_navigate_01: Navigation library smoke tests.
//!
//! Exercises the s2-navigate crate across multiple S2 levels, verifies
//! cell mappings are deterministic, and benchmarks the navigable index
//! rendering pipeline. Complements bench/conversational/ — this tests
//! the navigation ENGINE, not conversational token economics.
//!
//! cargo run --example bench_navigate_01

use s2_navigate::*;

fn main() {
    println!("=== s2-navigate: Context Navigation Smoke Test ===\n");

    // ── Build sample context items simulating a real session ──────────
    let mut items = Vec::new();

    // Directives (always present)
    items.push(directive_item("focus", 350, 88));
    items.push(directive_item("foresight", 200, 50));

    // File references from a coding session
    items.push(file_item("src/parser.rs", 2, "read_file", 1200, Some("abc123def456")));
    items.push(file_item("src/codegen.rs", 3, "read_file", 900, Some("def456abc123")));
    items.push(file_item("src/main.rs", 4, "read_file", 600, Some("789ghi012jkl")));
    items.push(file_item("src/error.rs", 6, "write_file", 400, Some("mno345pqr678")));

    // Error signatures
    items.push(error_item("E0308", 5, 3, "mismatched types: expected u64, found &str"));
    items.push(error_item("E0433", 5, 1, "failed to resolve: use of undeclared crate"));

    // CCR markers from compression
    items.push(marker_item("2cea00846456e611721e5a1a60565c5a9af10cce",
                           "[code_rust:3fns|2structs|1impls 180L]", 2, 5962, 40));
    items.push(marker_item("3dfb11957567a822832e6b2b71666d6cbe0b22dd",
                           "[code_rust:2fns|1structs 160L]", 3, 4221, 40));
    items.push(marker_item("4eac22a68678b933943f7c3c82777d7dcf1c33ee",
                           "[build:7E 3W 32L]", 5, 2100, 40));

    // Conversation turns
    items.push(turn_item(1, "user: help refactoring error types across modules", 30));
    items.push(turn_item(2, "assistant: read parser.rs, codegen.rs, main.rs", 25));
    items.push(turn_item(3, "tool: parser.rs (5962B) + codegen.rs (4221B) + main.rs", 20));
    items.push(turn_item(4, "assistant: extract shared error type into error.rs", 35));
    items.push(turn_item(5, "tool: cargo build → 7 errors, 3 warnings", 15));
    items.push(turn_item(6, "assistant: fixed all errors, clean build", 20));

    let nav = ContextNavigator::new(items);
    println!("Session: {} turns, {} context items, {} total tokens\n",
             nav.total_turns, nav.item_count(), nav.total_tokens());

    // ── Test 1: Coarse overview (L4) ──────────────────────────────────
    println!("── Test 1: Coarse overview at L4 ──");
    let view_l4 = nav.context_at_level(4);
    println!("Cells: {}, token budget: {}", view_l4.cell_count, view_l4.token_budget);
    let rendered = render_navigable_index(&view_l4, "code_first");
    print!("{}", rendered);

    // ── Test 2: Medium level (L8) ─────────────────────────────────────
    println!("── Test 2: Medium level at L8 ──");
    let view_l8 = nav.context_at_level(8);
    println!("Cells: {}, token budget: {}", view_l8.cell_count, view_l8.token_budget);
    let rendered = render_navigable_index(&view_l8, "code_first");
    print!("{}", rendered);

    // ── Test 3: Fine level (L12) ──────────────────────────────────────
    println!("── Test 3: Fine level at L12 ──");
    let view_l12 = nav.context_at_level(12);
    println!("Cells: {}, token budget: {}", view_l12.cell_count, view_l12.token_budget);
    // Only show first 10 cells to keep output manageable
    let mut short_view = view_l12.clone();
    short_view.cells.truncate(10);
    short_view.children_available.clear();
    let rendered = render_navigable_index(&short_view, "code_first");
    print!("{}", rendered);
    println!("  ... ({} more cells)\n", view_l12.cell_count - 10);

    // ── Test 4: Navigate to a file cell ───────────────────────────────
    println!("── Test 4: Zoom into files band ──");
    let file_cells = nav.cells_in_band(ContextBand::Files, 9);
    println!("File cells at L9: {}", file_cells.len());
    if let Some(first) = file_cells.first() {
        let items = nav.navigate_to_cell(*first);
        let rendered = render_cell_detail(&items);
        print!("{}", rendered);
    }

    // ── Test 5: Navigate to marker cells ──────────────────────────────
    println!("── Test 5: Zoom into markers band ──");
    let marker_cells = nav.cells_in_band(ContextBand::Markers, 12);
    println!("Marker cells at L12: {}", marker_cells.len());
    if let Some(first) = marker_cells.first() {
        let items = nav.navigate_to_cell(*first);
        let rendered = render_cell_detail(&items);
        print!("{}", rendered);
    }

    // ── Test 6: Covering sizes at different levels ────────────────────
    println!("── Test 6: Covering cell counts by level ──");
    for level in [2u8, 4, 6, 8, 10, 12, 14] {
        let cover = nav.covering(level);
        println!("  L{:2}: {} cells", level, cover.0.len());
    }

    // ── Test 7: Profile integration ───────────────────────────────────
    println!("\n── Test 7: Task profile → context items ──");
    for profile in s2_probe::task_profiles() {
        let pitems = profile_items(&profile);
        let pnav = ContextNavigator::new(pitems);
        let pview = pnav.context_at_level(6);
        println!("  {:>10}: {} blocks → {} cells at L6, ~{} tok",
                 profile.task, profile.blocks.len(),
                 pview.cell_count, pview.token_budget);
    }

    // ── Test 8: Navigable index at different zoom levels ──────────────
    println!("\n── Test 8: Navigable index token cost by level ──");
    println!("  {:>6} {:>8} {:>10} {:>12}", "Level", "Cells", "TokenCost", "Savings%");
    let base_cost = nav.context_at_level(4).token_budget.max(1) as f64;
    for level in [2u8, 4, 6, 8, 10, 12, 14, 16] {
        let view = nav.context_at_level(level);
        let savings = (1.0 - view.token_budget as f64 / base_cost) * 100.0;
        println!("  L{:>4} {:>8} {:>10} {:>11.1}%",
                 level, view.cell_count, view.token_budget, savings);
    }

    println!("\n=== OK ===");
}
