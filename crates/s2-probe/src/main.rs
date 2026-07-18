// s2-probe: S2 as a LEVEL GENERATOR for different SHAPES of context, used to
// analyze and figure out better PLACEMENTS for context blocks.
//
// The user's framing (2026-07-18): "look at S2 as a level generator for
// different shapes of context, since this is how we'd like to analyse and
// figure out better placements for context blocks."
//
// What that means concretely:
//   - A context block (directives / recall catalog / plain-data / conversation)
//     has a token budget. We can render it at many S2 *levels*: low level =
//     a few big coarse cells (cheap, blurry shape); high level = many small
//     fine cells (expensive, sharp shape). The level GENERATES the shape.
//   - Different *placements* (which latitude band, which longitude slice = where
//     in the window) of the same blocks produce different superimposed shapes.
//   - We score each placement on contiguity / contention / locality / balance
//     and recommend the best one. That's "figure out better placements."
//
// This binary is a demo driver; the logic lives in lib.rs with unit tests.

use s2_probe::{
    analyze_placements, generate_block_shapes, recommend, Block, Placement, Scorer,
};

fn main() {
    // The Aphrodite per-turn context blocks with approximate token budgets
    // (from report 08 P6's char-cost breakdown; tokens ≈ chars/4).
    let blocks = [
        Block::new("system",  120,  3),   // system prompt: small, coarse
        Block::new("directives", 420, 5),
        Block::new("nudges",   80,  7),
        Block::new("plain",   1000, 8),
        Block::new("recall",  1200, 10),  // catalog: big, fine
        Block::new("hint",     60,  6),
        Block::new("convo",   1600, 12),  // conversation: biggest, finest
    ];

    // 1. LEVEL GENERATOR - sweep S2 levels for one block, show how the SHAPE
    // changes with level. This is "S2 as a level generator for different shapes."
    println!("=== LEVEL GENERATOR: shape of 'recall' (1200 tok) across S2 levels ===");
    let shapes = generate_block_shapes(&blocks[4]);
    for sh in &shapes {
        println!(
            "  L{:2}: {:4} cells, area={:7.4}, contig={:.3}, span={:.1}°x{:.1}°",
            sh.level, sh.cells, sh.area, sh.contiguity, sh.lng_span, sh.lat_span,
        );
    }
    // Find the "knee": the lowest level where the shape is still contiguous and
    // bounded - the cheapest adequate shape.
    let knee = shapes
        .iter()
        .find(|s| s.contiguity > 0.9 && s.cells <= 64)
        .map(|s| s.level)
        .unwrap_or(99);
    println!("  -> recommended level for 'recall' (knee): L{}", knee);

    // 2. PLACEMENT ANALYZER - score candidate placements of all blocks.
    println!("\n=== PLACEMENT ANALYZER: score candidate placements ===");
    let placements = analyze_placements(&blocks);
    let scorer = Scorer::new();
    let mut scored: Vec<(Placement, f64)> = placements
        .iter()
        .map(|p| (p.clone(), scorer.score(p)))
        .collect();
    scored.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());

    println!(
        "{:28} {:>8} {:>8} {:>8} {:>8} {:>8}",
        "placement", "contig", "noCont", "local", "balnce", "SCORE",
    );
    for (p, s) in &scored {
        let (c, n, l, b) = scorer.parts(p);
        println!(
            "{:28} {:8.3} {:8.3} {:8.3} {:8.3} {:8.3}",
            p.name, c, n, l, b, s,
        );
    }

    // 3. RECOMMEND - the best placement.
    let best = recommend(&blocks, &scorer);
    println!("\n=== RECOMMENDED placement: {} (score {:.3}) ===", best.name, scorer.score(&best));
    for (blk, slot) in best.blocks.iter().zip(0..) {
        println!(
            "  slot {} lat[{:+5.1},{:+5.1}] lng[{:5.1},{:5.1}]  {} @ L{}",
            slot, blk.lat_lo, blk.lat_hi, blk.lng_lo, blk.lng_hi, blk.name, blk.level,
        );
    }

    // 4. WHY this placement wins: print its contiguity / contention / locality /
    //    balance so the user can see what "better" means.
    let (c, n, l, b) = scorer.parts(&best);
    println!(
        "\n  contiguity={:.3} (one region per block), no-contention={:.3}, locality={:.3} (Hilbert-adjacent), balance={:.3}",
        c, n, l, b,
    );
    println!("\nSee lib.rs unit tests for the level generator + scorer + analyzer invariants.");
}
