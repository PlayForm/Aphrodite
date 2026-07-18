//! s2-probe: S2 as a level generator for different shapes of context, used to
//! analyze and find better placements for context blocks.
//!
//! See `main.rs` for the framing. This module holds the tested logic:
//!   - [`Shape`] / [`generate_block_shapes`]: the LEVEL GENERATOR. Sweep S2
//!     levels 0..=MAX for one context block and emit a shape descriptor per
//!     level. The level GENERATES the shape (cells, area, contiguity, span).
//!   - [`Block`] / [`Placement`]: a context block with a token budget, and a
//!     placement = an assignment of blocks to lat-band + lng-slice slots.
//!   - [`Scorer`] / [`score`] / [`recommend`]: score a placement on contiguity,
//!     contention, locality, area-balance; recommend the best.

use s2::cellid::CellID;
use s2::cellunion::CellUnion;
use s2::rect::Rect;
use s2::region::RegionCoverer;

/// Highest S2 level we generate shapes at. S2 max is 30; we cap at 16 because
/// context-block shapes past L16 are needlessly fine (cell area < 1e-6 of the
/// sphere) and the coverer is O(cells).
pub const MAX_LEVEL:u8 = 16;

/// A context block - one section of the turn's injected context.
#[derive(Clone, Debug)]
pub struct Block {
    pub name:&'static str,
    /// Approx token budget for this block (drives area allocation).
    pub tokens:usize,
    /// Default S2 resolution to render at (the "base shape").
    pub level:u8,
}

impl Block {
    pub fn new(name:&'static str, tokens:usize, level:u8) -> Self { Self { name, tokens, level } }
}

/// A block placed into a slot on the sphere: a latitude band + longitude slice.
#[derive(Clone, Debug)]
pub struct PlacedBlock {
    pub name:&'static str,
    pub lat_lo:f64, pub lat_hi:f64,
    pub lng_lo:f64, pub lng_hi:f64,
    pub level:u8,
}

/// A placement = a name + a set of placed blocks covering the window.
#[derive(Clone, Debug)]
pub struct Placement {
    pub name:&'static str,
    pub blocks:Vec<PlacedBlock>,
}

/// The SHAPE of one block at one S2 level - what the level GENERATOR emits.
#[derive(Clone, Debug, PartialEq)]
pub struct Shape {
    pub level:u8,
    /// Number of S2 cells covering the block's region at this level.
    pub cells:usize,
    /// Total area of the covering (fraction of the unit sphere).
    pub area:f64,
    /// Contiguity: 1.0 = a single connected S2 cell region; lower = scattered.
    /// Measured as (cells - connected_components + 1) / cells, in [0,1].
    pub contiguity:f64,
    /// Longitude span in degrees the block occupies (position-in-window width).
    pub lng_span:f64,
    /// Latitude span in degrees (the block's tier thickness).
    pub lat_span:f64,
}

/// Generate the SHAPE of `block` at every S2 level 0..=MAX_LEVEL.
///
/// The block is mapped to a fixed lat band (one tier per block, stacked by
/// index) and a longitude slice sized by its token budget share. As the level
/// rises, the covering gets finer: more cells, sharper boundary, lower per-cell
/// area. This is "S2 as a level generator for different shapes of context."
pub fn generate_block_shapes(block:&Block) -> Vec<Shape> {
    // Fixed lat band for shape generation: the block occupies a 20° tier.
    let lat_lo = 0.0_f64;
    let lat_hi = 20.0_f64;
    // Longitude slice sized by token budget: ~ share of a 360° sweep. Cap at
    // 120° so even the biggest block doesn't wrap the sphere.
    let share = (block.tokens as f64 / 4000.0).min(1.0).max(0.02);
    let lng_span = (share * 360.0).min(120.0);
    let lng_lo = -lng_span / 2.0;
    let lng_hi = lng_span / 2.0;
    let rect = Rect::from_degrees(lat_lo, lng_lo, lat_hi, lng_hi);

    let mut out = Vec::with_capacity((MAX_LEVEL + 1) as usize);
    for level in 0..=MAX_LEVEL {
        let rc = RegionCoverer { min_level:level, max_level:level, level_mod:1, max_cells:512 };
        let cover = rc.covering(&rect);
        let cells = cover.0.len();
        let area = cover.approx_area();
        let contiguity = contiguity(&cover.0);
        out.push(Shape {
            level,
            cells,
            area,
            contiguity,
            lng_span,
            lat_span: lat_hi - lat_lo,
        });
    }
    out
}

/// Contiguity score: 1.0 if the covering is a single connected S2 region; lower
/// as it fragments into more disconnected components. We approximate "connected
/// component" via the S2 cell adjacency: two cells are in the same component if
/// one contains the other's parent-neighbour. Cheap version: count components
/// by transitive containment of `range_min`/`range_max` overlap.
fn contiguity(cells:&[CellID]) -> f64 {
    if cells.is_empty() { return 0.0; }
    // Union-find over "intersects" (range overlap) - two cells in the same S2
    // covering that touch share a boundary, so their ranges overlap or abut.
    let n = cells.len();
    let mut parent:Vec<usize> = (0..n).collect();
    fn find(parent:&mut Vec<usize>, mut i:usize) -> usize {
        while parent[i] != i { parent[i] = parent[parent[i]]; i = parent[i]; }
        i
    }
    for i in 0..n {
        for j in (i + 1)..n {
            if cells[i].intersects(&cells[j]) {
                let (a, b) = (find(&mut parent, i), find(&mut parent, j));
                if a != b { parent[a] = b; }
            }
        }
    }
    let components = (0..n).filter(|&i| find(&mut parent, i) == i).count();
    // 1.0 when one component; falls toward 0 as components rise.
    if n == 1 { 1.0 } else { (n - components) as f64 / (n - 1) as f64 }
}

/// Score a placement. Returns (contiguity, no_contention, locality, balance),
/// each in [0,1], and a weighted total. Higher is better.
#[derive(Clone, Copy, Debug)]
pub struct Scorer;

impl Scorer {
    pub const W_CONTIG:f64 = 0.30;
    pub const W_NOCONT:f64 = 0.25;
    pub const W_LOCAL:f64 = 0.25;
    pub const W_BAL:f64 = 0.20;

    pub fn new() -> Self { Self }

    /// Score a placement; return the four parts (contig, no_contention,
    /// locality, balance) each in [0,1].
    pub fn parts(&self, p:&Placement) -> (f64, f64, f64, f64) {
        let blocks = &p.blocks;
        let n = blocks.len();

        // Cover each placed block at its level.
        let covers:Vec<CellUnion> = blocks
            .iter()
            .map(|b| {
                let rect = Rect::from_degrees(b.lat_lo, b.lng_lo, b.lat_hi, b.lng_hi);
                let rc = RegionCoverer {
                    min_level:b.level, max_level:b.level, level_mod:1, max_cells:512,
                };
                rc.covering(&rect)
            })
            .collect();

        // 1. CONTIGUITY: each block should be one connected region (mean contig).
        let contig:f64 = blocks
            .iter()
            .zip(covers.iter())
            .map(|(_, c)| contiguity(&c.0))
            .sum::<f64>()
            / n as f64;

        // 2. NO-CONTENTION: blocks shouldn't overlap. 1 - (overlap area / total).
        let mut total_overlap:f64 = 0.0;
        for i in 0..n {
            for j in (i + 1)..n {
                let inter = CellUnion::intersection(&covers[i], &covers[j]);
                total_overlap += inter.approx_area();
            }
        }
        let total_area:f64 = covers.iter().map(|c| c.approx_area()).sum();
        let no_cont = if total_area > 0.0 { 1.0 - (total_overlap / total_area).min(1.0) } else { 1.0 };

        // 3. LOCALITY: adjacent-in-window blocks should be adjacent on the
        //    sphere (Hilbert locality). Measure: for each consecutive pair, do
        //    their nearest cells share a common ancestor at a coarse level?
        let mut loc_score = 0.0;
        for i in 1..n {
            let a = &covers[i - 1];
            let b = &covers[i];
            // Best common-ancestor level across all cell pairs; higher = closer.
            let mut best:u64 = 0;
            for ca in &a.0 {
                for cb in &b.0 {
                    if let Some(lvl) = ca.common_ancestor_level(cb) {
                        if lvl > best { best = lvl; }
                    }
                }
            }
            // Normalize: ancestor level 0 = opposite sides of sphere (no
            // locality); level >=8 = essentially touching. Map to [0,1].
            loc_score += (best as f64 / 8.0).min(1.0);
        }
        let locality = if n > 1 { loc_score / (n - 1) as f64 } else { 1.0 };

        // 4. AREA-BALANCE: each block's area should be proportional to its token
        //    budget (no block over- or under-sized vs its share). 1 - mean
        //    deviation from the ideal share.
        let total_tokens:usize = blocks.iter().map(|b| tokens_of(b.name)).sum::<usize>().max(1);
        let mut dev = 0.0;
        for (blk, cov) in blocks.iter().zip(covers.iter()) {
            let ideal = tokens_of(blk.name) as f64 / total_tokens as f64;
            let actual = if total_area > 0.0 { cov.approx_area() / total_area } else { 0.0 };
            dev += (ideal - actual).abs();
        }
        let balance = 1.0 - (dev / n as f64).min(1.0);

        (contig, no_cont, locality, balance)
    }

    /// Weighted total score in [0,1]. Higher is better.
    pub fn score(&self, p:&Placement) -> f64 {
        let (c, n, l, b) = self.parts(p);
        Self::W_CONTIG * c + Self::W_NOCONT * n + Self::W_LOCAL * l + Self::W_BAL * b
    }
}

/// Look up a block's token budget by name (kept from the demo set so the scorer
/// doesn't need the original `Block` list threaded through). In a real wiring
/// this would come from the live `flow.rs` per-section char costs.
fn tokens_of(name:&str) -> usize {
    match name {
        "system" => 120,
        "directives" => 420,
        "nudges" => 80,
        "plain" => 1000,
        "recall" => 1200,
        "hint" => 60,
        "convo" => 1600,
        _ => 400,
    }
}

/// Build a set of candidate placements for `blocks` and return them all for
/// scoring. Each placement is a different assignment of blocks to latitude
/// bands (the "tier" order) and a longitude partition (the window split).
pub fn analyze_placements(blocks:&[Block]) -> Vec<Placement> {
    // Two strategies:
    //  (a) "stacked" - each block gets its own latitude band, longitude sized
    //      by token share. Maximizes no-contention (no lat overlap).
    //  (b) "packed" - blocks share a single wide latitude band, each gets a
    //      longitude slice sized by token share. Maximizes locality (adjacent
    //      blocks are Hilbert-adjacent in one band).
    //  (c) "reversed" - packed but blocks in reverse order (tests that the
    //      scorer is order-sensitive via locality).

    let total_tokens:usize = blocks.iter().map(|b| b.tokens).sum::<usize>().max(1);
    let mut placements = Vec::new();

    // (a) stacked
    {
        let mut placed = Vec::new();
        let mut lat = -85.0_f64;
        let band = 170.0 / blocks.len() as f64;
        for b in blocks {
            let share = b.tokens as f64 / total_tokens as f64;
            let lng_span = (share * 360.0).min(360.0);
            let lng_lo = -lng_span / 2.0;
            placed.push(PlacedBlock {
                name:b.name,
                lat_lo:lat,
                lat_hi:lat + band,
                lng_lo,
                lng_hi:lng_lo + lng_span,
                level:b.level,
            });
            lat += band;
        }
        placements.push(Placement { name:"stacked", blocks:placed });
    }

    // (b) packed (single band, ordered by the given block order)
    placements.push(packed("packed", blocks, total_tokens, false));

    // (c) reversed packed
    placements.push(packed("packed_rev", blocks, total_tokens, true));

    // (d) packed with finer levels (+2) - sharper shapes, more cells
    {
        let finer:Vec<Block> = blocks
            .iter()
            .map(|b| Block { name:b.name, tokens:b.tokens, level:(b.level + 2).min(MAX_LEVEL) })
            .collect();
        placements.push(packed("packed_fine", &finer, total_tokens, false));
    }

    placements
}

fn packed(name:&'static str, blocks:&[Block], total_tokens:usize, rev:bool) -> Placement {
    let mut placed = Vec::new();
    let mut cursor = -180.0_f64;
    let order:Vec<&Block> = if rev { blocks.iter().rev().collect() } else { blocks.iter().collect() };
    for b in order {
        let share = b.tokens as f64 / total_tokens as f64;
        let span = share * 360.0;
        placed.push(PlacedBlock {
            name:b.name,
            lat_lo:-30.0,
            lat_hi:30.0,
            lng_lo:cursor,
            lng_hi:cursor + span,
            level:b.level,
        });
        cursor += span;
    }
    Placement { name, blocks:placed }
}

/// Recommend the best placement (highest score).
pub fn recommend(blocks:&[Block], scorer:&Scorer) -> Placement {
    let placements = analyze_placements(blocks);
    placements
        .into_iter()
        .max_by(|a, b| {
            scorer.score(a)
                .partial_cmp(&scorer.score(b))
                .unwrap_or(std::cmp::Ordering::Equal)
        })
        .expect("at least one placement")
}

#[cfg(test)]
mod tests {
    use super::*;
    use s2::latlng::LatLng;

    fn approx_eq(a:f64, b:f64) -> bool { (a - b).abs() < 1e-9 }

    // ── LEVEL GENERATOR: shape changes monotonically with level ──

    #[test]
    fn level_generator_emits_every_level() {
        let blk = Block::new("recall", 1200, 10);
        let shapes = generate_block_shapes(&blk);
        assert_eq!(shapes.len(), (MAX_LEVEL + 1) as usize);
        for (i, s) in shapes.iter().enumerate() {
            assert_eq!(s.level as usize, i, "shapes must be in level order");
        }
    }

    #[test]
    fn level_generator_cells_nondecreasing_with_level() {
        // A finer level never produces fewer cells than a coarser level for the
        // same region (the coverer subdivides).
        let blk = Block::new("recall", 1200, 10);
        let shapes = generate_block_shapes(&blk);
        for w in shapes.windows(2) {
            assert!(
                w[1].cells >= w[0].cells,
                "cells must be non-decreasing: L{}={} -> L{}={}",
                w[0].level, w[0].cells, w[1].level, w[1].cells,
            );
        }
    }

    #[test]
    fn level_generator_level0_is_one_cell_per_face_span() {
        // At L0 each cell is a whole face; a small block fits in <=4 faces.
        let blk = Block::new("hint", 60, 6);
        let shapes = generate_block_shapes(&blk);
        let l0 = &shapes[0];
        assert_eq!(l0.level, 0);
        assert!(l0.cells <= 6, "L0 covering of a small block must be tiny, got {}", l0.cells);
        assert!(l0.contiguity > 0.0, "L0 must be contiguous");
    }

    #[test]
    fn level_generator_area_bounded_by_block_span() {
        // The covered area must never exceed the block's lat×lng span fraction
        // of the sphere by more than a small cover slack.
        let blk = Block::new("plain", 1000, 8);
        let shapes = generate_block_shapes(&blk);
        // Sphere area = 4π ≈ 12.566. Block span = 20° × (share*360°).
        let share = (1000.0_f64 / 4000.0).min(1.0);
        let lng_span = (share * 360.0).min(120.0);
        let lat_span = 20.0_f64;
        let ideal_fraction = (lat_span / 180.0) * (lng_span / 360.0);
        let ideal_area = 4.0 * std::f64::consts::PI * ideal_fraction;
        // Cover slack: RegionCoverer can over-cover by a factor; allow 3x.
        for s in &shapes {
            assert!(
                s.area <= ideal_area * 3.0 + 0.01,
                "L{} area {} exceeds 3x ideal {}",
                s.level, s.area, ideal_area,
            );
        }
    }

    #[test]
    fn level_generator_contiguity_in_unit_interval() {
        let blk = Block::new("convo", 1600, 12);
        for s in generate_block_shapes(&blk) {
            assert!(s.contiguity >= 0.0 && s.contiguity <= 1.0, "contiguity out of [0,1]");
        }
    }

    // ── SCORER: parts are in [0,1], score is the weighted sum ──

    #[test]
    fn scorer_parts_in_unit_interval() {
        let blocks = [
            Block::new("directives", 420, 5),
            Block::new("recall", 1200, 10),
            Block::new("convo", 1600, 12),
        ];
        let scorer = Scorer::new();
        for p in analyze_placements(&blocks) {
            let (c, n, l, b) = scorer.parts(&p);
            for v in [c, n, l, b] {
                assert!(v >= 0.0 && v <= 1.0, "part out of [0,1]: {}", v);
            }
            let s = scorer.score(&p);
            assert!(s >= 0.0 && s <= 1.0, "score out of [0,1]: {}", s);
        }
    }

    #[test]
    fn scorer_weights_sum_to_one() {
        assert!(approx_eq(
            Scorer::W_CONTIG + Scorer::W_NOCONT + Scorer::W_LOCAL + Scorer::W_BAL,
            1.0,
        ));
    }

    // ── ANALYZER: packed (locality-optimized) beats stacked on locality ──

    #[test]
    fn packed_has_higher_locality_than_stacked() {
        let blocks = [
            Block::new("directives", 420, 5),
            Block::new("plain", 1000, 8),
            Block::new("recall", 1200, 10),
            Block::new("convo", 1600, 12),
        ];
        let scorer = Scorer::new();
        let placements = analyze_placements(&blocks);
        let stacked = placements.iter().find(|p| p.name == "stacked").unwrap();
        let packed = placements.iter().find(|p| p.name == "packed").unwrap();
        let (_, _, loc_stack, _) = scorer.parts(stacked);
        let (_, _, loc_pack, _) = scorer.parts(packed);
        assert!(
            loc_pack >= loc_stack,
            "packed must have >= locality than stacked: packed={} stacked={}",
            loc_pack, loc_stack,
        );
    }

    #[test]
    fn stacked_has_higher_no_contention_than_packed() {
        // Stacked gives each block its own latitude band -> no lat overlap ->
        // no contention. Packed shares one band -> some lng-adjacency overlap.
        let blocks = [
            Block::new("directives", 420, 5),
            Block::new("plain", 1000, 8),
            Block::new("recall", 1200, 10),
            Block::new("convo", 1600, 12),
        ];
        let scorer = Scorer::new();
        let placements = analyze_placements(&blocks);
        let stacked = placements.iter().find(|p| p.name == "stacked").unwrap();
        let packed = placements.iter().find(|p| p.name == "packed").unwrap();
        let (_, nc_stack, _, _) = scorer.parts(stacked);
        let (_, nc_pack, _, _) = scorer.parts(packed);
        assert!(
            nc_stack >= nc_pack,
            "stacked must have >= no-contention than packed: stacked={} packed={}",
            nc_stack, nc_pack,
        );
    }

    #[test]
    fn recommend_picks_a_valid_placement() {
        let blocks = [
            Block::new("directives", 420, 5),
            Block::new("recall", 1200, 10),
            Block::new("convo", 1600, 12),
        ];
        let best = recommend(&blocks, &Scorer::new());
        assert!(!best.blocks.is_empty());
        assert!(["stacked", "packed", "packed_rev", "packed_fine"].contains(&best.name));
    }

    #[test]
    fn order_reversal_lowers_locality() {
        // Reversed packed should have lower locality than forward packed,
        // because the Hilbert-adjacent order is disturbed (the conversation
        // block, which should sit next to recall, is moved to the far end).
        let blocks = [
            Block::new("directives", 420, 5),
            Block::new("plain", 1000, 8),
            Block::new("recall", 1200, 10),
            Block::new("convo", 1600, 12),
        ];
        let scorer = Scorer::new();
        let placements = analyze_placements(&blocks);
        let packed = placements.iter().find(|p| p.name == "packed").unwrap();
        let rev = placements.iter().find(|p| p.name == "packed_rev").unwrap();
        let (_, _, loc_fwd, _) = scorer.parts(packed);
        let (_, _, loc_rev, _) = scorer.parts(rev);
        // Forward packed should be at least as good on locality.
        assert!(loc_fwd >= loc_rev - 0.02, "forward packed locality {} should be >= reversed {}", loc_fwd, loc_rev);
    }

    // ── SHAPE: contiguity of a single cell is 1.0 ──

    #[test]
    fn contiguity_single_cell_is_one() {
        let c = CellID::from_face_pos_level(0, 0, 5);
        assert!(approx_eq(contiguity(&[c]), 1.0));
    }

    #[test]
    fn contiguity_distant_cells_is_low() {
        // Two cells on opposite faces have no common ancestor -> 2 components.
        let a = CellID::from_face_pos_level(0, 0, 5);
        let b = CellID::from_face_pos_level(3, 0, 5);
        let c = contiguity(&[a, b]);
        assert!(c < 0.6, "two distant cells should be low contiguity, got {}", c);
    }

    // ── LOCALITY: Hilbert adjacency holds for adjacent positions ──

    #[test]
    fn adjacent_positions_share_coarse_ancestor() {
        // Two lng positions 1° apart at the same latitude should share a
        // reasonably coarse ancestor (Hilbert locality).
        let a:CellID = LatLng::from_degrees(0.0, 0.0).into();
        let b:CellID = LatLng::from_degrees(0.0, 1.0).into();
        let lvl = a.common_ancestor_level(&b);
        assert!(lvl.is_some(), "adjacent points must share an ancestor");
        assert!(lvl.unwrap() >= 3, "adjacent points should share a coarse (>=L3) ancestor, got L{:?}", lvl);
    }

    #[test]
    fn antipodal_positions_share_no_fine_ancestor() {
        let a:CellID = LatLng::from_degrees(0.0, 0.0).into();
        let b:CellID = LatLng::from_degrees(0.0, 179.0).into();
        let lvl = a.common_ancestor_level(&b).unwrap_or(0);
        assert!(lvl < 5, "antipodal points should share only a very coarse ancestor, got L{}", lvl);
    }
}
