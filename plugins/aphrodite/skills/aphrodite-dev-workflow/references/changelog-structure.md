# CHANGELOG Structure

Every release entry must be multi-line with proper markdown subheadings:

```markdown
## Aphrodite Binary

### Features & Enhancements
#### v0.5.55 / 1.61.0 — 2026-06-16
- headroom cache/benchmark/token modes
- 1-8 workers
- Hermes default provider

### Bug Fixes
#### Critical
#### v0.5.56 / 1.62.2 — 2026-06-16
- C1: version bump, binary 0.5.52→0.5.56
- C3: turn_counter reset via module reference
- C4: engine marker format <<<CCR:hash|type|size>>>

#### High
#### v0.5.61 / 1.62.7 — 2026-06-16
- headroom CCR regex fix
- loopback exempt paths
- threshold inversion

### Infrastructure
#### v0.5.58 / 1.62.4 — 2026-06-16
- benchmark suite (19/19, 0.9ms avg)
- rust-toolchain.toml
- CHANGELOG.md created

## Plugin Version History
(keep existing table)
```

Rules:
- Use ### for categories (Features, Bug Fixes, Infrastructure)
- Use #### for individual releases with full version pairs
- Split each commit message comma-separated fixes into individual - bullets
- Every release under Bug Fixes gets grouped by severity
- Compare links mandatory on Bug Fix releases
- Keep Headroom section separate
- Plugin version history table always included at bottom
