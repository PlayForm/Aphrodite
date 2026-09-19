# DOC-REWRITE-PUBLIC-2026-09-19 - public docs/ rewrite step engine

Task (user, 2026-09-19): completely rewrite the public docs as they are stale.
Repo `/Volumes/CORSAIR/Developer/macOS/Application/PlayForm/Aphrodite`, branch
`Development`, binary 1.4.6 / plugin 2.1.4. NO COMMITS (auto-committer may
sweep - verify with `git log`/`git status`, never fight it). Public-facing
content must be free of internal process artifacts (no HANDOFF/STEP_ENGINE
references, no task-queue sections, no feedback-note phrasing, no
`tree/Development` links - default branch is `Current`).

## Verified ground facts (2026-09-19, from source)

- Default branch: `Current` (docs must link `tree/Current`, not
  `tree/Development`).
- Binary 1.4.6, plugin 2.1.4 (`plugin.yaml`, `BINARY_VERSION`, both Cargo.toml).
- Hooks: SIX (`on_session_start`, `pre_tool_call`, `transform_tool_result`,
  `transform_terminal_output`, `pre_llm_call`, `post_llm_call`) -
  `crates/aphrodite-hermes/src/lib.rs:335-462`, registered per-hook in
  `plugins/aphrodite/__init__.py:1090,1175`.
- `download.sh` / `download.ps1` STILL exist in `plugins/aphrodite/` (plugin
  submodule) - install docs referencing them are valid, but the flow must
  match the canonical runtime home `~/.hermes/aphrodite/` (binaries,
  directives, ccr.db, logs) with `layout_check.py` self-heal.
- `aphrodite setup` subcommand exists (`main.rs:47` pre-runtime dispatch).
- Removed in 1.4.6, verify ABSENT before claiming: `.githooks/`, `profiles/`,
  S2 navigation (`s2-probe`/`s2-navigate`/`aphrodite_navigate`), `list_skills`
  FFI, shipped skills (dev-side only, `.hermes/skills/`), `Maintain/install.sh`.
- New in 1.4.6 (document it): chain-split CCR engine (opt-in,
  `chain_split = false` shipped default), honest previews (`preview_max_chars`
  wired end-to-end, env > TOML > default 120), generated `_bindings.py`
  (cbindgen → ctypesgen → finalize), dylib subprocess probe, layout self-heal,
  `~/.hermes/aphrodite` canonical runtime home.
- Authoritative source to borrow/adapt: `.hermes/uml/` (11 verified flow
  traces with mermaid diagrams, v1.4.6), `.hermes/examples/` (real captured
  sessions), `.hermes/release-notes/v1.4.0..v1.4.6`, `.hermes/notes/ARCHITECTURE.md`.

## Target structure (docs/ mimics .hermes/ categories)

| New path                    | Source / notes                                                                                       |
| --------------------------- | ---------------------------------------------------------------------------------------------------- |
| docs/README.md              | Index (orchestrator-owned, rewritten LAST after links settle)                                        |
| docs/architecture/          | From `.hermes/uml/` 01-11: mermaid flow traces, scrubbed to public cleanliness                       |
| docs/classification/        | Content-type taxonomy (from `docs/ccr/content-types.md` + `.hermes/classification/TAXONOMY.md`)      |
| docs/ccr/                   | marker-format, lifecycle, backends/ (sqlite, in-memory, inline) - rewrite + verify                   |
| docs/examples/              | From `.hermes/examples/` + `docs/examples/llm-view.md` - real captures, token economics              |
| docs/release-notes/         | From `.hermes/release-notes/v1.4.0..v1.4.6` (new)                                                    |
| docs/install/               | README, macos-linux, windows, troubleshooting - rewrite for runtime home + `download.sh/.ps1`        |
| docs/config/                | aphrodite-toml (verify vs `aphrodite.toml.example`), env-vars                                        |
| docs/api/                   | health, metrics-endpoint, retrieve, ccr-endpoints                                                    |
| docs/metrics/               | prometheus, queries                                                                                  |
| docs/proxy/                 | architecture, handlers, retry, compression                                                           |
| docs/plugin/                | hooks (SIX), directives, context-engine                                                              |
| docs/tool-relay/            | tools (13), callbacks                                                                                |
| docs/guides/ (new)          | hermes-integration, hermes-tool-output-schemas, troubleshooting notes (from existing top-level docs) |
| docs/centers.md             | Roadmap - keep, label shipped vs sketch                                                              |

## Stale-content audit targets (per user: delete stale content)

- `docs/agent-feedback.md` - internal feedback-note artifact, likely DELETE or
  full scrub to functionality-only (user: public content free of feedback
  notes). Audit pair decides with evidence.
- `docs/APHRODITE-HEADROOM.md`, `docs/HEADROOM-FORK-DIFF.md` - fork-divergence
  docs; v1.4.6 release notes say "removed from the release line". Audit pair
  decides keep-as-guide vs move to `.hermes/notes/` (internal).
- `docs/hermes-tool-output-schemas.md` - classifier playbook; internal-ish,
  audit pair decides.

## Wave plan (pairs of subagents, disjoint file ownership)

- Wave 1: PAIR-1 architecture (01-06 / 07-11+README), PAIR-2 ccr + classification
- Wave 2: PAIR-3 config + install, PAIR-4 proxy + api/metrics
- Wave 3: PAIR-5 plugin + tool-relay, PAIR-6 examples + release-notes
- Wave 4: PAIR-7 stale-content audit (agent-feedback + APHRODITE-HEADROOM /
  HEADROOM-FORK-DIFF + tool-output-schemas), PAIR-8 guides/ migration
- Orchestrator (LAST, after all agents): docs/README.md index + root README.md
  + ALL link updates (tree/Current), prettier + link + mermaid verification.

## Per-child contract (verbatim in every brief)

1. Read the existing doc, verify EVERY claim against source (grep real
   file:line, run the command if cheap). Claim still true → keep (rewrite
   clean). Claim false/stale → correct AND append to this file's discrepancy
   log (section below).
2. Diagrams: mermaid fences must be valid (balanced ```mermaid blocks, valid
   mermaid syntax, no `file:line` inside diagrams that breaks rendering);
   ASCII diagrams stay inside code fences.
3. Public cleanliness: no internal process artifacts, no `tree/Development`
   links (use `tree/Current`), no file:line citations as "proof" in prose
   (style guide: docs describe behavior directly).
4. Style: docs/README.md style guide - explain then detail, tables over
   prose, no placeholder content, roadmap ideas labeled.
5. Ownership: ONLY the files named in the brief. No git commit/push/tag.
   Scratch goes to `.hermes/tmp/` if needed, never /tmp.
6. Finish: `npx prettier --check` on every touched .md (repo .prettierrc:
   tabs, width 100, proseWrap preserve).

## Discrepancy log (append-only, per child)

Children append their verified-stale findings here as `- [date] file: claim
X was stale; source says Y (file:line)`.