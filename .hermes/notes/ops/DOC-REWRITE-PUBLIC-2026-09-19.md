# DOC-REWRITE-PUBLIC-2026-09-19 - public docs/ rewrite step engine

Task (user, 2026-09-19): completely rewrite the public docs as they are stale.
Repo `/Volumes/CORSAIR/Developer/macOS/Application/PlayForm/Aphrodite`, branch
`Development`, binary 1.4.6 / plugin 2.1.4 (version values are snapshots of
that date - read `plugin.yaml`, `BINARY_VERSION`, Cargo.toml for live
values). NO COMMITS (auto-committer may
sweep - verify with `git log`/`git status`, never fight it). Public-facing
content must be free of internal process artifacts (no HANDOFF/STEP_ENGINE
references, no task-queue sections, no feedback-note phrasing). Link branch
rule (user correction 2026-09-19, refined same day): the branch in a repo
link is a PER-FILE property - each file is a halted process, and the branch
you find it on (and the branch it will be viewed from) decides its links:
a file living on the Development working line gets `tree/Development`, a
file living on the Current distribution line gets `tree/Current`. Infer per
file with `git symbolic-ref --short HEAD` (or the file's position in the
dual line) at edit time - never blanket-assume one branch for every file
(see ground facts below; the original task text said "no tree/Development
links - default branch is Current", which the user explicitly reversed).

## Verified ground facts (2026-09-19, from source)

- Default branch: `Current` on GitHub, but the working branch is
  `Development`. The link branch is a PER-FILE property: a file's links
  carry the branch the file itself lives on (halted-process rule) -
  Development files → `tree/Development`, Current files →
  `tree/Current`. During this rewrite the docs/ tree lives on
  Development, so its links are `tree/Development` - that is the outcome
  of per-file inference, not a blanket rule (user correction 2026-09-19:
  links follow the branch the repo is developed on).
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

| New path             | Source / notes                                                                                       |
| -------------------- | ---------------------------------------------------------------------------------------------------- |
| docs/README.md       | Index (orchestrator-owned, rewritten LAST after links settle)                                        |
| docs/architecture/   | From `.hermes/uml/` 01-11: mermaid flow traces, scrubbed to public cleanliness                       |
| docs/classification/ | Content-type taxonomy (from `docs/ccr/content-types.md` + `.hermes/classification/TAXONOMY.md`)      |
| docs/ccr/            | marker-format, lifecycle, backends/ (sqlite, in-memory, inline) - rewrite + verify                   |
| docs/examples/       | From `.hermes/examples/` + `docs/examples/llm-view.md` - real captures, token economics              |
| docs/release-notes/  | From `.hermes/release-notes/v1.4.0..v1.4.6` (new)                                                    |
| docs/install/        | README, macos-linux, windows, troubleshooting - rewrite for runtime home + `download.sh/.ps1`        |
| docs/config/         | aphrodite-toml (verify vs `aphrodite.toml.example`), env-vars                                        |
| docs/api/            | health, metrics-endpoint, retrieve, ccr-endpoints                                                    |
| docs/metrics/        | prometheus, queries                                                                                  |
| docs/proxy/          | architecture, handlers, retry, compression                                                           |
| docs/plugin/         | hooks (SIX), directives, context-engine                                                              |
| docs/tool-relay/     | tools (13), callbacks                                                                                |
| docs/guides/ (new)   | hermes-integration, hermes-tool-output-schemas, troubleshooting notes (from existing top-level docs) |
| docs/centers.md      | Roadmap - keep, label shipped vs sketch                                                              |

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
    - ALL link updates (tree/Development), prettier + link + mermaid verification.

## Per-child contract (verbatim in every brief)

1. Read the existing doc, verify EVERY claim against source (grep real
   file:line, run the command if cheap). Claim still true → keep (rewrite
   clean). Claim false/stale → correct AND append to this file's discrepancy
   log (section below).
2. Diagrams: mermaid fences must be valid (balanced ```mermaid blocks, valid
   mermaid syntax, no `file:line` inside diagrams that breaks rendering);
   ASCII diagrams stay inside code fences.
3. Public cleanliness: no internal process artifacts, no branch links
   that contradict the file's own branch context - infer the branch per
   file at edit time (`git symbolic-ref --short HEAD`; the branch the
   file was halted on / will be viewed from decides the link, so a file
   on the Development working line gets `tree/Development`, never a
   blanket `tree/Current` or hardcoded rule), no file:line citations as
   "proof" in prose
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

## Resume status (2026-09-19 afternoon, orchestrator)

Resumed by orchestrator after the morning session died on provider 429/401s.

## COMPLETE 2026-09-19 (orchestrator final pass done)

- [x] Wave 4 PAIR-7-A/B: stale audit verdicts (A + B files in
      .hermes/notes/ops/AUDIT-PAIR7-{A,B}-2026-09-19.md). Executed:
      agent-feedback.md + APHRODITE-HEADROOM.md + HEADROOM-FORK-DIFF.md moved
      to .hermes/notes/; centers.md 2 stale Python rows corrected (center is
      Rust-only); hermes-tool-output-schemas.md rewritten into docs/guides/
- [x] Wave 4 PAIR-8: docs/guides/ created (README + hermes-integration +
      hermes-tool-output-schemas), old top-level files removed
- [x] Orchestrator LAST: docs/README.md index rewritten (relative links, 30
      types, 6 hooks, 13 tools, 8 handlers, all categories incl. release-notes + guides); root README.md 7 tree/Development links fixed + 2
      moved-internal Headroom refs re-pointed; marker-format preview-order
      flag resolved (doc was correct: proxy=preview-first, hook=marker-first,
      verified proxy.rs:1917 vs tools.rs:273). Verification: 0 tree/Development
      links, 0 dead links, 27 mermaid fences 0 risks, prettier clean on all
      .hermes/**/*.md + docs + README. Discrepancy logs: 13 files (~110
      entries). NO COMMITS by orchestrator (auto-committer swept waves 1-4).

- [x] Wave 1 PAIR-1-A: docs/architecture/01-06 (landed morning, verified on disk + git status)
- [x] Wave 1 PAIR-2-C: docs/ccr/ backends + lifecycle + marker-format (landed
      morning, verified on disk)
- [x] Wave 1 PAIR-1-B: docs/architecture/07-11 + README (landed deleg_5f42b944,
      787s, prettier-clean, 3 discrepancy entries)
- [x] Wave 1 PAIR-2-D: docs/classification/ (landed deleg_1bd50bd5, 417s; REAL
      content-type count = 30 not 26; content-types.md moved; 6 discrepancy
      entries)
- [x] Wave 2 PAIR-3-A/B: docs/config/ + docs/install/ (landed: A deleg_abec2877
      956s 15 disc; B deleg_e505e0b8 530s 10 disc; both prettier-clean)
- [x] Wave 2 PAIR-4-A/B: docs/proxy/ + docs/api/ + docs/metrics/ (landed
      deleg_6e117e0c: A 723s 18 disc, B 539s 9 disc; both prettier-clean)
- [x] Wave 3 PAIR-5-A/B: docs/plugin/ + docs/tool-relay/ (landed deleg_e4b3f0f2:
      A 608s 13 disc hooks=SIX, B 654s 16 disc tools=13; both prettier-clean)
- [x] Wave 3 PAIR-6-A/B: docs/examples/ + docs/release-notes/ (landed
      deleg_72ca1bf0: A 465s 12 disc, B 252s; both prettier-clean. Flag:
      marker-format.md preview-first claim vs real marker-preview relay -
      orchestrator final pass)
- [~] Wave 4 PAIR-7-A/B: stale-content audit READ-ONLY (RUNNING deleg_b7c7d65c)
- [ ] Wave 2 PAIR-4-A/B: docs/proxy/ + docs/api/ + docs/metrics/ (briefs
      staged .hermes/tmp/DOC-REWRITE-BRIEF-PAIR4-A.md / PAIR4-B.md)
- [ ] Wave 3 PAIR-5-A/B: docs/plugin/ + docs/tool-relay/ (briefs staged
      .hermes/tmp/DOC-REWRITE-BRIEF-PAIR5-A.md / PAIR5-B.md)
- [ ] Wave 3 PAIR-6-A/B: docs/examples/ + docs/release-notes/ (briefs staged
      .hermes/tmp/DOC-REWRITE-BRIEF-PAIR6-A.md / PAIR6-B.md)
- [ ] Wave 4 PAIR-7-A/B: stale-content audit (read-only verdicts; briefs
      staged .hermes/tmp/DOC-REWRITE-BRIEF-PAIR7-A.md / PAIR7-B.md)
- [ ] Wave 4 PAIR-8: docs/guides/ migration (brief staged
      .hermes/tmp/DOC-REWRITE-BRIEF-PAIR8.md)
- [ ] Orchestrator LAST: docs/README.md index + root README.md + ALL link
      updates (tree/Development), prettier + link + mermaid verification.

Orchestrator mermaid pass already done on docs/examples/llm-view.md Scenario
4: labels quoted (["..."]) - fixes the DIAMOND_START parse error; the x4
times-symbol was replaced with plain x4. All 17 mermaid fences in docs/ now
pass the risk scan.
