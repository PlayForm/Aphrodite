# Release Notes Template

Two modes. Pick one per release, based on whether you can actually build and
test the commit being described.

- **Live mode** - you're cutting this release right now: the working tree at
  this commit is buildable, testable, on your machine. Include a real
  `### Infrastructure` section with commands you actually ran.
- **Retrospective mode** - you're rewriting notes for a release that already
  shipped, possibly long ago. Do NOT claim to have rebuilt or retested it.
  Replace `### Infrastructure` with `### Verification` describing how you
  derived the notes (commit range analyzed, diffstat), and `### What Ships`
  reflects only what was actually attached to the GitHub release at the time
  - never invent artifacts that weren't there.

Replace `{BIN_VERSION}` / `{PLUGIN_VERSION}` / `{PREV_VERSION}` with actual
values. Omit `Plugin v{PLUGIN_VERSION}` from the title and `plugins/aphrodite`
references entirely for releases that predate the Hermes plugin's existence.

---

## Live mode

```markdown
**[Compare {PREV_VERSION}...{BIN_VERSION}](https://github.com/PlayForm/Aphrodite/compare/{PREV_VERSION}...Aphrodite/{BIN_VERSION})**

## Aphrodite {BIN_VERSION} 💋 Plugin v{PLUGIN_VERSION}

### Summary

One paragraph. What this release is, why it matters, 2-3 sentences max.

### Changes

- **Feature**: description
- **Fix**: description
- **Chore**: description
- **Docs**: description

### Infrastructure

- Build: `cargo build --workspace --release` ✅
- Tests: `cargo test --workspace` ✅ (NNN passed, 0 failed)
- Python tests: `ruff check` + `pyright` ✅
- Live smoke test: `hermes -z` end-to-end ✅ (only if actually run)

### What Ships

| Artifact | Platform |
|----------|----------|
| `aphrodite-aarch64-apple-darwin` | macOS ARM64 |
| `aphrodite-x86_64-apple-darwin` | macOS Intel |
| `aphrodite-x86_64-unknown-linux-gnu` | Linux x86_64 |
| `aphrodite-x86_64-pc-windows-msvc` | Windows x86_64 |
| Plugin v{PLUGIN_VERSION} | Hermes (standalone repo) |

> **Always list all four targets above - do NOT trim from a live asset count.**
> `Build.yml`'s matrix always produces the full set, and the `Finalize` job
> fails the release if any platform is missing. Windows is the slow leg, so at
> authoring time only 9/12 assets may be attached yet - that is a timing race,
> not a missing Windows build. Never write "no Windows release": either it's
> coming (wait for `Build` to finish / the tag's run to conclude) or the build
> genuinely failed (then `Finalize` is red - fix the build, don't ship the note).

### Links

- **Full Changelog**: https://github.com/PlayForm/Aphrodite/compare/{PREV_VERSION}...Aphrodite/{BIN_VERSION}
- **CHANGELOG.md**: [Maintain/CHANGELOG.md](Maintain/CHANGELOG.md)
- **Plugin**: https://github.com/PlayForm/Aphrodite-Hermes
- **Headroom Fork**: https://github.com/PlayForm/Headroom
```

## Retrospective mode (rewriting a past release's notes)

```markdown
**[Compare {PREV_VERSION}...{BIN_VERSION}](https://github.com/PlayForm/Aphrodite/compare/{PREV_VERSION}...{BIN_VERSION})**

## Aphrodite {BIN_VERSION}[ 💋 Plugin v{PLUGIN_VERSION}]

### Summary

One paragraph, written from reading the actual commits in this range - not
copied from the original notes. What shipped, why it mattered at the time.

### Changes

- **Feature**: description
- **Fix**: description
- **Chore**: description
- **Docs**: description

### Verification

- Commit range analyzed: `{PREV_SHA}..{THIS_SHA}` (N commits)
- Diffstat: N files changed, +X/-Y lines
- (Optional) Notable risk/rollback context if the original commit messages flag one

### What Ships

Only if the historical GitHub release actually had assets attached - list them
as they were. Omit this section entirely rather than fabricate artifacts.

### Links

- **Full Changelog**: https://github.com/PlayForm/Aphrodite/compare/{PREV_VERSION}...{BIN_VERSION}
```

---

## Section Rules

1. **Summary** - always present, 1-3 sentences, grounded in the actual diff (not the old notes' wording, not guessed).
2. **Changes** - bullet list grouped by Feature/Fix/Chore/Docs. Omit empty categories. Every bullet must trace to a real commit in range.
3. **Infrastructure** (live) - always present when you can actually run the commands. Never claim a check you didn't run.
4. **Verification** (retrospective) - always present. States what was analyzed, not what was re-tested.
5. **What Ships** - live: list the full fixed platform matrix (all four targets + plugin), NOT a point-in-time asset snapshot - the `Build.yml` matrix always produces them and the `Finalize` job enforces completeness, so a partial attach at authoring time is a race, never a missing platform. Retrospective: list only what that historical release actually shipped, or omit (older releases predate some targets - do not backfill artifacts that were never there).
6. **Links** - always present. Minimum: compare link. Add CHANGELOG/plugin/fork links when relevant to that release.
7. Never emit a bare compare-link-only body. That's the anti-pattern this template exists to fix.

## Anti-Pattern (DO NOT)

```
**Full Changelog**: https://github.com/PlayForm/Aphrodite/compare/Aphrodite/v0.8.42...Aphrodite/v0.8.43
```

A bare compare link with zero description tells readers nothing about what changed. Every release gets a real Summary and Changes list, even a short one.
