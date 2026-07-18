// s2-probe: graph the per-task context shapes and store them all.
//
// The user's framing (2026-07-18): S2 is a level generator for different
// shapes of context. The goal is NOT to find better placements - it is to
// GRAPH each task's context shape and keep a DATABASE of all the different
// shapes as they accumulate.
//
// Usage: s2-probe [shape-db-path]   (default: .bench/s2-shapes.jsonl)

use s2_probe::{generate_block_shapes, render_task, task_profiles, ShapeStore};

fn main() {
    let db_path = std::env::args().nth(1).unwrap_or_else(|| ".bench/s2-shapes.jsonl".to_string());
    let store = ShapeStore::open(&db_path).expect("open shape store");

    for profile in task_profiles() {
        let r = render_task(&profile);

        // 1. THE GRAPH - one lat band per block, lng = position in window,
        // glyph = the block's S2 level (hex digit). Coarse blocks render as
        // sparse low digits, fine blocks as dense high digits.
        println!("=== task: {} ({} tok) ===", r.task, r.total_tokens);
        print!("{}", r.grid);

        // 2. THE SUPERIMPOSITION - normalized union of every block's cover,
        // differentiated by cell level.
        let hist:Vec<String> = r
            .level_histogram
            .iter()
            .map(|(l, c)| format!("L{}:{}", l, c))
            .collect();
        println!("  superimposition: {} cells [{}]", r.union_cells, hist.join(" "));

        // 3. THE DATABASE - persist this task's shapes at their native levels
        // plus the full level sweep per block (all the shapes each block CAN
        // take), so the corpus accumulates across runs.
        let mut batch = r.shapes.clone();
        for b in &profile.blocks {
            batch.extend(generate_block_shapes(profile.task, b));
        }
        let n = store.append(&batch).expect("append shapes");
        println!("  stored {} shapes -> {}\n", n, store.path().display());
    }

    println!("=== shape database summary ===");
    match store.summary() {
        Ok(rows) => {
            let total:usize = rows.iter().map(|(_, c)| c).sum();
            for (task, count) in &rows {
                println!("  {:>10}: {:5} shapes", task, count);
            }
            println!("  {:>10}: {:5} shapes total (all runs)", "ALL", total);
        }
        Err(e) => println!("  (summary unavailable: {})", e),
    }
}
