//! Preview test battery - the frozen contract for what the model sees.
//!
//! The preview is the final, honest representation of the stored payload, so a
//! changed string in this battery is a contract break and not a cosmetic one.
//! The battery is split by area: `Robustness` for pathological input, `Cap` for
//! the length budget, `Arm` for exact per-arm output, `Honesty` for the
//! never-misleading pins and `Detect` for shape detection and upgrade routing.
//! Every area shares the fixtures and the cap guard declared here.

use super::*;

/// Serializes tests that mutate the process-global preview cap (cargo runs this
/// module's tests concurrently; the cap is process-wide).
fn CapGuard() -> std::sync::MutexGuard<'static, ()> { crate::Preview::PreviewCapTestGuard() }

const GitStatus:&str = " M crates/aphrodite/src/preview.rs\n M crates/aphrodite/src/hooks.rs\nA  src/new_a.rs\nA  \
                        src/new_b.rs\nD  src/old.rs\n?? tmp/scratch\n?? tmp/other\n?? build/log";

const CargoTest:&str = "running 221 tests\ntest foo::bar ... ok\ntest result: ok. 220 passed; 0 failed; 1 ignored; 0 \
                        measured; 0 filtered out; finished in 0.31s";

const LsLong:&str = "total 48\ndrwxr-xr-x  5 nikola staff  160 Jul 14 10:00 src\ndrwxr-xr-x  2 nikola staff   64 Jul \
                     14 10:00 tests\n-rw-r--r--  1 nikola staff 1913 Jul 14 10:00 preview.rs\n-rw-r--r--  1 nikola \
                     staff  820 Jul 14 10:00 hooks.rs\n-rw-r--r--  1 nikola staff  512 Jul 14 10:00 README.md";

const Ripgrep:&str = "src/preview.rs:12:    let lines = content.lines().count();\nsrc/preview.rs:88:    \
                      format!(\"[terminal...\nsrc/hooks.rs:91:    let preview = \
                      crate::build_preview();\nsrc/marker.rs:49:    let mut safe = preview.replace();";

const GitLog:&str = "commit abc1234def5678\nAuthor: Nikola <n@x.io>\nDate:   Mon Jul 14\n\n    fix(preview): stop \
                     doubling\n\ncommit def5678abc1234\nAuthor: Nikola <n@x.io>\nDate:   Sun Jul 13\n\n    feat: add \
                     detector";

mod Arm;
mod Cap;
mod Detect;
mod Honesty;
mod Robustness;
