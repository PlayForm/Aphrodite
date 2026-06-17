---
name: aphrodite-presentation
description:
    "How to present Aphrodite features in README, docs, and user-facing output —
    framing, naming, value-prop patterns."
version: 1.0.0
platforms: [macos]
---

# Aphrodite Presentation & Docs

Rules for presenting Aphrodite features to users (developers evaluating the
proxy).

## README Framing

Every "before/after" comparison MUST be framed as the user's agent experience —
never as internal refactoring.

| ❌ Don't                                  | ✅ Do                                       |
| ----------------------------------------- | ------------------------------------------- |
| `Before → After`                          | `Without Aphrodite → With Aphrodite`        |
| Column: `result[:120]`                    | Column: `Your agent sees without Aphrodite` |
| Section: `Token savings per content type` | Section: `Your agent's context budget`      |
| `Preview tokens vs Full expansion`        | `Without Aphrodite vs With Aphrodite`       |

**Rule**: every "before" column is the agent's experience today. Every "after"
is what they get with Aphrodite. The reader is evaluating adoption — show value,
not engineering.

## Value Prop Language

- Never say "we changed X to Y" — say "your agent sees Y instead of X"
- Never present internal refactors as features — "`_ESSENTIAL_TOOLS` refactor"
  is noise to a user
- Frame architecture as division of labor: "Aphrodite owns the addressing layer;
  Headroom owns the reduction layer"
- Use concrete token savings: "23× fewer tokens" not "preview vs full expansion
  savings"

## Badge & Version Consistency

- Release badge, plugin version badge, and `BIN_VERSION` / `PLUGIN_VERSION` must
  match
- Update badges in README on every release
- Binary version tracks Cargo.toml; plugin version tracks plugin.yaml

## Pitfalls

- **Internal refactor as feature**: presenting a code cleanup as a user-facing
  change. Filter: would a developer choosing Aphrodite care about
  `_ESSENTIAL_TOOLS`? No — show what tools get compressed instead.
- **Abstract savings**: "median 23× savings" without context. Always anchor to
  concrete examples (git diff, build output, etc.)
- **Stale badges**: version badges that don't match the latest release — the
  first thing a visitor checks.
