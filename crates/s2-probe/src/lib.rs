//! s2-probe: S2 as a level generator for different shapes of context.
//!
//! The point is NOT to optimize placements - it is to GRAPH the shapes and
//! KEEP them: render how each task's context window looks as an S2 covering
//! (level = resolution tier, longitude = position in window, latitude band =
//! block), and persist every generated shape into a shape database so the
//! collection of shapes across tasks/levels accumulates and can be compared
//! later.
//!
//!   - [`Block`] / [`TaskProfile`] / [`task_profiles`]: per-task context-block
//!     mixes (a debug turn, an explore turn, ... have different budgets and
//!     resolutions - so different shapes).
//!   - [`Shape`] / [`generate_block_shapes`]: the LEVEL GENERATOR. Sweep S2
//!     levels 0..=MAX for one block and emit a shape descriptor per level.
//!   - [`render_task`] / [`TaskRender`]: the GRAPH. ASCII-render the task's
//!     superimposed context shape (one lat band per block, lng = position in
//!     window, glyph = S2 level as hex digit) plus the normalized-union level
//!     histogram (the superimposition, differentiated by `cell.level()`).
//!   - [`ShapeStore`]: the DATABASE. Append-only JSONL of every [`Shape`]
//!     generated, queryable by task/block across runs.

use std::io::{BufRead, Write};
use std::path::PathBuf;

use s2::cellid::CellID;
use s2::cellunion::CellUnion;
use s2::latlng::LatLng;
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
    /// Approx token budget for this block (drives the lng span = window share).
    pub tokens:usize,
    /// S2 resolution to render at. The level IS the shape's grain.
    pub level:u8,
}

impl Block {
    pub fn new(name:&'static str, tokens:usize, level:u8) -> Self { Self { name, tokens, level } }
}

/// A task's context-window profile: the block mix a given kind of agent turn
/// actually carries. Different tasks weight the same blocks differently, so
/// each task renders to a visibly different shape.
#[derive(Clone, Debug)]
pub struct TaskProfile {
    pub task:&'static str,
    pub blocks:Vec<Block>,
}

impl TaskProfile {
    pub fn total_tokens(&self) -> usize { self.blocks.iter().map(|b| b.tokens).sum() }
}

/// The built-in per-task profiles. Budgets are the Aphrodite per-turn blocks
/// (tokens ~ chars/4, from report 08 P6's char-cost breakdown), skewed per
/// task the way the matching directive skews them (focus/explore/cleanup...).
pub fn task_profiles() -> Vec<TaskProfile> {
    vec![
        TaskProfile {
            task:"baseline",
            blocks:vec![
                Block::new("system", 120, 3),
                Block::new("directives", 420, 5),
                Block::new("nudges", 80, 7),
                Block::new("plain", 1000, 8),
                Block::new("recall", 1200, 10),
                Block::new("hint", 60, 6),
                Block::new("convo", 1600, 12),
            ],
        },
        TaskProfile {
            // focus directive: deep retrieval + fine-grained error context.
            task:"debug",
            blocks:vec![
                Block::new("system", 120, 3),
                Block::new("directives", 500, 6),
                Block::new("nudges", 160, 9),
                Block::new("plain", 600, 8),
                Block::new("recall", 1800, 11),
                Block::new("hint", 60, 6),
                Block::new("convo", 2000, 13),
            ],
        },
        TaskProfile {
            // explore directive: broad plain data + wide recall, thin convo.
            task:"explore",
            blocks:vec![
                Block::new("system", 120, 3),
                Block::new("directives", 380, 5),
                Block::new("nudges", 40, 6),
                Block::new("plain", 1600, 9),
                Block::new("recall", 2000, 10),
                Block::new("hint", 60, 6),
                Block::new("convo", 800, 10),
            ],
        },
        TaskProfile {
            // foresight directive: heavy directive programming, balanced rest.
            task:"implement",
            blocks:vec![
                Block::new("system", 120, 3),
                Block::new("directives", 600, 7),
                Block::new("nudges", 120, 8),
                Block::new("plain", 1200, 9),
                Block::new("recall", 1000, 10),
                Block::new("hint", 60, 6),
                Block::new("convo", 1400, 11),
            ],
        },
        TaskProfile {
            // review: diff-heavy plain data dominates.
            task:"review",
            blocks:vec![
                Block::new("system", 120, 3),
                Block::new("directives", 450, 6),
                Block::new("nudges", 80, 7),
                Block::new("plain", 1800, 10),
                Block::new("recall", 800, 9),
                Block::new("hint", 60, 6),
                Block::new("convo", 1000, 10),
            ],
        },
        TaskProfile {
            // cleanup directive: everything small and coarse.
            task:"cleanup",
            blocks:vec![
                Block::new("system", 120, 3),
                Block::new("directives", 300, 5),
                Block::new("nudges", 60, 6),
                Block::new("plain", 400, 7),
                Block::new("recall", 600, 8),
                Block::new("hint", 60, 5),
                Block::new("convo", 600, 9),
            ],
        },
    ]
}

/// The SHAPE of one block at one S2 level - what the level GENERATOR emits and
/// what the [`ShapeStore`] persists. Flat and serde-friendly by hand (no serde
/// dep: the JSONL codec below is 9 fixed fields).
#[derive(Clone, Debug, PartialEq)]
pub struct Shape {
    pub task:String,
    pub block:String,
    pub tokens:usize,
    pub level:u8,
    /// Number of S2 cells covering the block's region at this level.
    pub cells:usize,
    /// Total area of the covering (fraction of the unit sphere).
    pub area:f64,
    /// Contiguity: 1.0 = a single connected S2 cell region; lower = scattered.
    pub contiguity:f64,
    /// Longitude span in degrees (position-in-window width).
    pub lng_span:f64,
    /// Latitude span in degrees (the block's tier thickness).
    pub lat_span:f64,
}

/// Generate the SHAPE of `block` at every S2 level 0..=MAX_LEVEL.
///
/// The block is mapped to a fixed 20-degree lat band and a longitude slice
/// sized by its token budget share. As the level rises the covering gets
/// finer: more cells, sharper boundary, smaller per-cell area. The level
/// GENERATES the shape.
pub fn generate_block_shapes(task:&str, block:&Block) -> Vec<Shape> {
    let lat_lo = 0.0_f64;
    let lat_hi = 20.0_f64;
    // Longitude slice sized by token budget: ~ share of a 360-degree sweep.
    // Cap at 120 degrees so even the biggest block doesn't wrap the sphere.
    let share = (block.tokens as f64 / 4000.0).min(1.0).max(0.02);
    let lng_span = (share * 360.0).min(120.0);
    let lng_lo = -lng_span / 2.0;
    let lng_hi = lng_span / 2.0;
    let rect = Rect::from_degrees(lat_lo, lng_lo, lat_hi, lng_hi);

    let mut out = Vec::with_capacity((MAX_LEVEL + 1) as usize);
    for level in 0..=MAX_LEVEL {
        let rc = RegionCoverer { min_level:level, max_level:level, level_mod:1, max_cells:512 };
        let cover = rc.covering(&rect);
        out.push(Shape {
            task:task.to_string(),
            block:block.name.to_string(),
            tokens:block.tokens,
            level,
            cells:cover.0.len(),
            area:cover.approx_area(),
            contiguity:contiguity(&cover.0),
            lng_span,
            lat_span:lat_hi - lat_lo,
        });
    }
    out
}

/// Contiguity score: 1.0 if the covering is a single connected S2 region;
/// lower as it fragments. Union-find over cell-range intersection.
fn contiguity(cells:&[CellID]) -> f64 {
    if cells.is_empty() { return 0.0; }
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
    if n == 1 { 1.0 } else { (n - components) as f64 / (n - 1) as f64 }
}

/// One rendered task graph: the ASCII grid, the level histogram of the
/// normalized superimposition, and the per-block shapes at their native level.
#[derive(Clone, Debug)]
pub struct TaskRender {
    pub task:String,
    pub total_tokens:usize,
    /// The ASCII graph: one 2-row lat band per block; glyph = level hex digit,
    /// `.` = uncovered. Longitude (columns) = position in the window.
    pub grid:String,
    /// (level, cell-count) of the normalized union of all block covers - the
    /// SUPERIMPOSITION, differentiated by `cell.level()`.
    pub level_histogram:Vec<(u8, usize)>,
    /// Total cells in the normalized superimposed union.
    pub union_cells:usize,
    /// The per-block shape at each block's native level, window-ordered.
    pub shapes:Vec<Shape>,
}

const GRID_COLS:usize = 72;
const ROWS_PER_BLOCK:usize = 2;

/// Graph one task's context shape.
///
/// Layout: blocks are laid out along longitude in window order (lng share =
/// token share of the full 360 sweep - position in window IS longitude), each
/// in its own latitude band stacked top-to-bottom. Each block is covered at
/// its own S2 level and drawn with that level's hex digit, so coarse blocks
/// read as sparse bands of small digits and fine blocks as dense bands of
/// high digits - the level differentiation is visible in the glyphs.
pub fn render_task(profile:&TaskProfile) -> TaskRender {
    let total_tokens = profile.total_tokens().max(1);
    let n = profile.blocks.len();
    let band = 120.0 / n as f64; // lat range +60..-60, one band per block

    let mut grid = String::new();
    let mut all_cells:Vec<CellID> = Vec::new();
    let mut shapes = Vec::new();
    let mut cursor = -180.0_f64;

    for (i, b) in profile.blocks.iter().enumerate() {
        let share = b.tokens as f64 / total_tokens as f64;
        let span = (share * 360.0).max(5.0);
        let (lng_lo, lng_hi) = (cursor, (cursor + span).min(180.0));
        cursor += span;
        let lat_hi = 60.0 - band * i as f64;
        let lat_lo = lat_hi - band;

        let rect = Rect::from_degrees(lat_lo, lng_lo, lat_hi, lng_hi);
        let rc = RegionCoverer {
            min_level:b.level, max_level:b.level, level_mod:1, max_cells:512,
        };
        let cover = rc.covering(&rect);
        let glyph = char::from_digit(b.level as u32, 32).unwrap_or('?');

        for row in 0..ROWS_PER_BLOCK {
            let lat = lat_lo + band * (0.25 + 0.5 * row as f64);
            let mut line = String::with_capacity(GRID_COLS);
            for col in 0..GRID_COLS {
                let lng = -180.0 + 360.0 * (col as f64 + 0.5) / GRID_COLS as f64;
                let c = CellID::from(LatLng::from_degrees(lat, lng)).parent(b.level as u64);
                line.push(if cover.0.contains(&c) { glyph } else { '.' });
            }
            if row == 0 {
                grid.push_str(&format!(
                    "{:>10} L{:<2}|{}| {:3} cells {:4} tok\n",
                    b.name, b.level, line, cover.0.len(), b.tokens,
                ));
            } else {
                grid.push_str(&format!("{:>13}|{}|\n", "", line));
            }
        }

        shapes.push(Shape {
            task:profile.task.to_string(),
            block:b.name.to_string(),
            tokens:b.tokens,
            level:b.level,
            cells:cover.0.len(),
            area:cover.approx_area(),
            contiguity:contiguity(&cover.0),
            lng_span:lng_hi - lng_lo,
            lat_span:band,
        });
        all_cells.extend(cover.0.iter().cloned());
    }

    // The superimposition: union everything, normalize (merges complete
    // sibling sets upward), histogram by resulting cell level.
    let mut union = CellUnion(all_cells);
    union.normalize();
    let mut level_histogram:Vec<(u8, usize)> = Vec::new();
    for c in &union.0 {
        let lvl = c.level() as u8;
        match level_histogram.iter_mut().find(|(l, _)| *l == lvl) {
            Some((_, count)) => *count += 1,
            None => level_histogram.push((lvl, 1)),
        }
    }
    level_histogram.sort_by_key(|(l, _)| *l);

    TaskRender {
        task:profile.task.to_string(),
        total_tokens,
        grid,
        union_cells:union.0.len(),
        level_histogram,
        shapes,
    }
}

/// Append-only JSONL database of generated [`Shape`]s. One line per shape;
/// shapes accumulate across runs so the collection of task/block/level shapes
/// grows into a comparable corpus. Codec is hand-rolled (9 fixed fields) to
/// keep the probe's dependency surface at exactly `s2`.
pub struct ShapeStore {
    path:PathBuf,
}

impl ShapeStore {
    /// Open (creating parent directories; the file itself is created lazily on
    /// first append).
    pub fn open(path:impl Into<PathBuf>) -> std::io::Result<Self> {
        let path = path.into();
        if let Some(dir) = path.parent() {
            if !dir.as_os_str().is_empty() { std::fs::create_dir_all(dir)?; }
        }
        Ok(Self { path })
    }

    pub fn path(&self) -> &std::path::Path { &self.path }

    /// Append shapes as JSONL. Returns the number written.
    pub fn append(&self, shapes:&[Shape]) -> std::io::Result<usize> {
        let mut f = std::fs::OpenOptions::new().create(true).append(true).open(&self.path)?;
        for s in shapes {
            writeln!(f, "{}", encode(s))?;
        }
        Ok(shapes.len())
    }

    /// Load every shape ever stored. Lines that fail to decode are skipped
    /// (forward-compat with future field additions).
    pub fn load(&self) -> std::io::Result<Vec<Shape>> {
        let f = match std::fs::File::open(&self.path) {
            Ok(f) => f,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
            Err(e) => return Err(e),
        };
        Ok(std::io::BufReader::new(f).lines().map_while(Result::ok).filter_map(|l| decode(&l)).collect())
    }

    /// Per-task shape counts, insertion-ordered - the database's table of
    /// contents.
    pub fn summary(&self) -> std::io::Result<Vec<(String, usize)>> {
        let mut out:Vec<(String, usize)> = Vec::new();
        for s in self.load()? {
            match out.iter_mut().find(|(t, _)| *t == s.task) {
                Some((_, count)) => *count += 1,
                None => out.push((s.task, 1)),
            }
        }
        Ok(out)
    }
}

fn encode(s:&Shape) -> String {
    format!(
        "{{\"task\":\"{}\",\"block\":\"{}\",\"tokens\":{},\"level\":{},\"cells\":{},\"area\":{:e},\"contiguity\":{},\"lng_span\":{},\"lat_span\":{}}}",
        s.task, s.block, s.tokens, s.level, s.cells, s.area, s.contiguity, s.lng_span, s.lat_span,
    )
}

fn decode(line:&str) -> Option<Shape> {
    fn str_field(line:&str, key:&str) -> Option<String> {
        let pat = format!("\"{}\":\"", key);
        let start = line.find(&pat)? + pat.len();
        let end = line[start..].find('"')? + start;
        Some(line[start..end].to_string())
    }
    fn num_field<T:std::str::FromStr>(line:&str, key:&str) -> Option<T> {
        let pat = format!("\"{}\":", key);
        let start = line.find(&pat)? + pat.len();
        let end = line[start..]
            .find(|c:char| c == ',' || c == '}')
            .map(|i| i + start)?;
        line[start..end].trim().parse().ok()
    }
    Some(Shape {
        task:str_field(line, "task")?,
        block:str_field(line, "block")?,
        tokens:num_field(line, "tokens")?,
        level:num_field(line, "level")?,
        cells:num_field(line, "cells")?,
        area:num_field(line, "area")?,
        contiguity:num_field(line, "contiguity")?,
        lng_span:num_field(line, "lng_span")?,
        lat_span:num_field(line, "lat_span")?,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── LEVEL GENERATOR ──

    #[test]
    fn level_generator_emits_every_level() {
        let shapes = generate_block_shapes("t", &Block::new("recall", 1200, 10));
        assert_eq!(shapes.len(), (MAX_LEVEL + 1) as usize);
        for (i, s) in shapes.iter().enumerate() {
            assert_eq!(s.level as usize, i);
            assert_eq!(s.task, "t");
            assert_eq!(s.block, "recall");
        }
    }

    #[test]
    fn cells_grow_with_level() {
        let shapes = generate_block_shapes("t", &Block::new("recall", 1200, 10));
        // Coarse levels can tie (1 cell covers everything), but from the point
        // the count starts moving it must never shrink as the level rises.
        for w in shapes.windows(2) {
            assert!(w[1].cells >= w[0].cells, "L{}={} < L{}={}", w[1].level, w[1].cells, w[0].level, w[0].cells);
        }
        assert!(shapes.last().unwrap().cells > shapes.first().unwrap().cells);
    }

    #[test]
    fn token_budget_drives_lng_span() {
        let small = generate_block_shapes("t", &Block::new("hint", 60, 6));
        let large = generate_block_shapes("t", &Block::new("convo", 1600, 6));
        assert!(large[6].lng_span > small[6].lng_span);
    }

    #[test]
    fn contiguity_single_cell_is_one() {
        assert_eq!(contiguity(&[CellID::from_face_pos_level(0, 0, 5)]), 1.0);
    }

    #[test]
    fn contiguity_disjoint_faces_is_zero() {
        let a = CellID::from_face_pos_level(0, 0, 5);
        let b = CellID::from_face_pos_level(3, 0, 5);
        assert_eq!(contiguity(&[a, b]), 0.0);
    }

    #[test]
    fn hilbert_locality_adjacent_positions_share_ancestors() {
        // Adjacent context positions (1 degree apart in-window) must be far
        // closer on the Hilbert curve than positions across the window.
        let a = CellID::from(LatLng::from_degrees(0.0, 0.0));
        let near = CellID::from(LatLng::from_degrees(0.0, 1.0));
        let far = CellID::from(LatLng::from_degrees(0.0, 179.0));
        let anc_near = a.common_ancestor_level(&near).unwrap_or(0);
        let anc_far = a.common_ancestor_level(&far).unwrap_or(0);
        assert!(anc_near > anc_far);
    }

    // ── TASK PROFILES + GRAPH ──

    #[test]
    fn profiles_cover_the_tasks() {
        let profiles = task_profiles();
        assert!(profiles.len() >= 5);
        for p in &profiles {
            assert!(!p.blocks.is_empty());
            assert!(p.total_tokens() > 0);
        }
        // Task skew is real: debug leans convo+recall, review leans plain.
        let get = |t:&str, b:&str| {
            profiles.iter().find(|p| p.task == t).unwrap()
                .blocks.iter().find(|x| x.name == b).unwrap().tokens
        };
        assert!(get("debug", "recall") > get("review", "recall"));
        assert!(get("review", "plain") > get("debug", "plain"));
    }

    #[test]
    fn render_draws_every_block_band() {
        let profiles = task_profiles();
        let r = render_task(&profiles[0]);
        assert_eq!(r.shapes.len(), profiles[0].blocks.len());
        assert_eq!(r.grid.lines().count(), profiles[0].blocks.len() * ROWS_PER_BLOCK);
        for b in &profiles[0].blocks {
            assert!(r.grid.contains(b.name), "band label {} missing", b.name);
            let glyph = char::from_digit(b.level as u32, 32).unwrap();
            assert!(r.grid.contains(glyph), "glyph {} for L{} missing", glyph, b.level);
        }
    }

    #[test]
    fn superimposition_histogram_is_consistent() {
        let r = render_task(&task_profiles()[0]);
        assert!(r.union_cells > 0);
        let total:usize = r.level_histogram.iter().map(|(_, c)| c).sum();
        assert_eq!(total, r.union_cells);
        // Normalization only merges upward: no level finer than the finest block.
        let max_block = r.shapes.iter().map(|s| s.level).max().unwrap();
        assert!(r.level_histogram.iter().all(|(l, _)| *l <= max_block));
    }

    #[test]
    fn different_tasks_render_different_shapes() {
        let profiles = task_profiles();
        let debug = render_task(profiles.iter().find(|p| p.task == "debug").unwrap());
        let cleanup = render_task(profiles.iter().find(|p| p.task == "cleanup").unwrap());
        assert_ne!(debug.grid, cleanup.grid);
        assert!(debug.union_cells > cleanup.union_cells, "finer task must superimpose to more cells");
    }

    // ── SHAPE DATABASE ──

    fn temp_store(tag:&str) -> ShapeStore {
        let path = std::env::temp_dir()
            .join(format!("s2-probe-test-{}-{}", std::process::id(), tag))
            .join("shapes.jsonl");
        let _ = std::fs::remove_file(&path);
        ShapeStore::open(path).unwrap()
    }

    #[test]
    fn store_round_trips_shapes() {
        let store = temp_store("roundtrip");
        let shapes = generate_block_shapes("debug", &Block::new("recall", 1800, 11));
        assert_eq!(store.append(&shapes).unwrap(), shapes.len());
        let loaded = store.load().unwrap();
        assert_eq!(loaded.len(), shapes.len());
        assert_eq!(loaded[11], shapes[11]);
        let _ = std::fs::remove_file(store.path());
    }

    #[test]
    fn store_accumulates_across_appends() {
        let store = temp_store("accumulate");
        for p in task_profiles() {
            let r = render_task(&p);
            store.append(&r.shapes).unwrap();
        }
        let summary = store.summary().unwrap();
        assert_eq!(summary.len(), task_profiles().len());
        for (task, count) in &summary {
            let blocks = task_profiles().iter().find(|p| p.task == task).unwrap().blocks.len();
            assert_eq!(*count, blocks, "task {} shape count", task);
        }
        let _ = std::fs::remove_file(store.path());
    }

    #[test]
    fn store_load_missing_file_is_empty() {
        let store = temp_store("missing");
        assert!(store.load().unwrap().is_empty());
        assert!(store.summary().unwrap().is_empty());
    }

    #[test]
    fn codec_survives_scientific_notation_area() {
        let mut s = generate_block_shapes("t", &Block::new("x", 100, 12))[12].clone();
        s.area = 3.5e-7;
        let back = decode(&encode(&s)).unwrap();
        assert!((back.area - s.area).abs() < 1e-12);
        assert_eq!(back, s);
    }
}
