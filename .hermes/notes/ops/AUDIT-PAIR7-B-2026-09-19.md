# AUDIT-PAIR7-B-2026-09-19 - stale-content audit (part B)

Repo: PlayForm/Aphrodite (branch Development, binary 1.4.6, plugin 2.1.4, default branch Current).
Audit: READ-ONLY. No docs/ file modified. Evidence verified against source on 2026-09-19.
Scope: docs/HEADROOM-FORK-DIFF.md, docs/hermes-tool-output-schemas.md, docs/centers.md.
Partner (A) audits docs/agent-feedback.md + docs/APHRODITE-HEADROOM.md in parallel.

## Verdict summary

| File                               | Verdict      | Destination                                             |
| ---------------------------------- | ------------ | ------------------------------------------------------- |
| docs/HEADROOM-FORK-DIFF.md         | MOVE-INTERNAL| .hermes/notes/HEADROOM-FORK-DIFF.md (internal archive)  |
| docs/hermes-tool-output-schemas.md | KEEP-PUBLIC  | rewrite functionality-only -> docs/guides/hermes-tool-output-schemas.md |
| docs/centers.md                    | KEEP-PUBLIC  | keep at docs/centers.md; fix 2 stale Python rows (L68-69) |

## Ground-fact cross-check (all verified against source)

- Hooks = SIX: on_session_start, pre_tool_call, transform_tool_result,
  transform_terminal_output, pre_llm_call, post_llm_call -
  crates/aphrodite-hermes/src/lib.rs:335,336,409,442,458,477.
- Default branch = Current: `git branch -a` shows `remotes/Source/HEAD -> Source/Current`; local `Current` exists.
- Runtime home ~/.hermes/aphrodite: plugins/aphrodite/__init__.py:35,145,155,163,299,317,343,501.
- Installers: plugins/aphrodite/download.sh + download.ps1 PRESENT; Maintain/install.sh ABSENT; .githooks/ ABSENT; profiles/ ABSENT.
- chain_split shipped default false: aphrodite.toml.example:71; docs/config/aphrodite-toml.md:118; .hermes/release-notes/v1.4.6.md:9.
- preview_max_chars shipped default 120: docs/ccr/marker-format.md:149; docs/config/aphrodite-toml.md:60,131; crates/aphrodite/src/proxy.rs:2645-2646.

---

## 1. docs/HEADROOM-FORK-DIFF.md (596 lines) -> MOVE-INTERNAL

Destination: `.hermes/notes/HEADROOM-FORK-DIFF.md` (internal engineering archive; .hermes/notes/ exists).

### INTERNAL evidence

- .hermes/release-notes/v1.4.6.md:54-56: fork-divergence docs (`APHRODITE-HEADROOM.md`,
  `HEADROOM-FORK-DIFF.md`, ...) were "removed from the release line" in 1.4.6 - the
  product already de-listed this doc from public distribution.
- Docs/HEADROOM-FORK-DIFF.md:236-239: "Aphrodite Fix-Pass Deltas ... executing
  `.plans/06-state-concurrency-storage.md`'s task list (uncommitted as of this
  writing - the executor contract requires recording any vendor edit here
  regardless of commit status)" - internal executor/planning-process language.
- Docs/HEADROOM-FORK-DIFF.md:93: "Per the 2026-07-11 lesson" - internal process recall.
- Docs/HEADROOM-FORK-DIFF.md:39: "the delta is queued for the next sync" - internal roadmap talk.
- Docs/HEADROOM-FORK-DIFF.md:229-232: "Hit and fixed one unrelated macOS Gatekeeper
  issue ... ad-hoc `codesign -s -` resolves it for local dev/test builds" - dev narrative.
- Entire body is vendor merge/conflict engineering history (commit hashes, conflict-
  resolution tables, silent-regression post-mortems) with zero user-facing value.

### STALE check: NOT stale on facts

- Branch claims verified: L31 "merge commit b1932981 on `Current`", L260 "All commits
  are on the `Current` branch" - Current branch confirmed present; no tree/Development links.
- L269 + L402-414 "Hermes Demo Suite (examples/hermes_demo/, 9 files, +1,653 lines)":
  PRESENT at vendor/headroom/examples/hermes_demo/ (doc paths are fork-relative).
- L420 "examples/recommendations.md (369)": PRESENT at vendor/headroom/examples/recommendations.md.
- L54-55 "new crates/headroom-simulators crate": PRESENT at vendor/headroom/crates/headroom-simulators.
- No hook-count, chain_split, preview_max_chars, or runtime-home claims to contradict.

Conclusion: accurate history, but internal by nature and already removed from the
release line -> MOVE-INTERNAL, not DELETE (retains engineering record) and not
KEEP-PUBLIC (process artifacts are structural, not cosmetic; scrubbing would gut the doc).

---

## 2. docs/hermes-tool-output-schemas.md (339 lines) -> KEEP-PUBLIC (rewrite)

Destination: `docs/guides/hermes-tool-output-schemas.md` (docs/guides/ is created by
PAIR-8; step engine target table names hermes-tool-output-schemas under docs/guides/).
Rewrite to functionality-only; scrub list below.

### STALE evidence

- L5 + L22: GitHub links use `tree/Development` - must be `tree/Development` (default branch is Current).
- L5: links `docs/ccr/content-types.md` - file ABSENT (docs/ccr/ holds only backends/,
  lifecycle.md, marker-format.md). Taxonomy now lives at docs/classification/content-types.md.
- L10 + L27: "22 classification types" - shipped taxonomy is larger: docs/classification/
  content-types.md carries 41 preview-shape rows (L122-145+) plus an 8-type base table
  (L39-45); PAIR-2-D recorded the real content-type count as 30. The 22-type table is stale.
- L48 lists `log` as shipped but L317 re-lists it under "New Classifier Types to Add";
  same self-contradiction for `write_file` (L49 shipped vs L325 to-add) and
  `browser_snapshot` (L50 shipped vs L333 to-add).

### INTERNAL evidence

- L7: references internal `Maintain/hermes_tool_output_formats.json` (dev-side tree;
  Maintain/install.sh was removed from the release line) - public docs must not point into Maintain/.
- L11: "The absorptive classifier uses this as its playbook" - classifier-internal framing.
- L14-15: "single source of truth for the absorptive CCR preview pipeline. When new
  tools or output shapes appear, they get documented here first - the classifier
  follows" - internal maintenance-workflow language, not user documentation.
- L315-339: "New Classifier Types to Add" - WIP/TODO section; must not ship publicly.

### Verified-OK evidence (keep in rewrite)

- 43 tools claim: Maintain/hermes_tool_output_formats.json contains exactly 43 entries.
- Enriched Preview Catalog anchor exists: docs/proxy/compression.md:213
  "## Enriched Preview Catalog" - the authoritative preview-format reference it defers to.
- Preview-shape examples (`[type:...]`) are consistent with the `[ct: metadata]` layout
  emitted by crates/aphrodite/src/proxy.rs:1915-1920 (proxy_format_ccr_output).

Rewrite requirements: fix tree/Development -> tree/Development (L5, L22); re-point L5 to
docs/classification/content-types.md; drop classifier-playbook framing (L9-16) and the
WIP section (L315-339); drop the Maintain/ path (L7) or describe it as internal-only;
align the taxonomy table with docs/classification/content-types.md or defer to it as
authoritative; keep the tool-by-tool shape/preview reference (functionality-only).

---

## 3. docs/centers.md (84 lines) -> KEEP-PUBLIC (fix 2 stale rows)

Destination: keep at `docs/centers.md` (step engine target table: "docs/centers.md |
Roadmap - keep, label shipped vs sketch"). Roadmap labeling is already honest (L3-6
disclaimer: only v1 implemented; v2-v4 are sketches) and matches the style rule.

### Shipped vs sketch labeling (verified against 1.4.6 source)

- v1 (current) - SHIPPED, Rust side verified:
  - L65 `format_ccr_output` `;center=X` in structure line: crates/aphrodite/src/proxy.rs:1915-1920
    (`let center_seg = center.map(|c| format!(";center={c}"))` -> `[{ct}: {metadata}{center_seg}]`).
  - L66 `smart_marker` `center: Option<&str>`: crates/aphrodite/src/proxy.rs:2069-2073.
  - L67 Rust tool relay `_ccr_center` from params: crates/aphrodite-hermes/src/tools.rs:305
    (`args.get("_ccr_center")`), schemas.rs:84 (schema exposure); marker.rs:39 (ccr_marker
    center param); marker format documented publicly at docs/ccr/marker-format.md:42,49-50,184.
- v2 Bucketed / v3 Accumulative / v4 center-as-filesystem - SKETCH, unimplemented:
  no bucket/accumulate/centers-fs code anywhere (grep over crates/aphrodite/src +
  crates/aphrodite-hermes/src found nothing). L3-6 disclaimer is correct; keep.

### STALE evidence (must fix in rewrite)

- L68: "Python `_ccr_marker` | `center=None` param" - no `_ccr_marker` symbol exists in
  plugins/aphrodite/ (only __init__.py, _bindings.py, layout_check.py; grep for
  `_ccr_marker`/`center` -> zero matches). STALE.
- L69: "Python `_compress_handler` | `X-Aphrodite-Center` header" - `X-Aphrodite-Center`
  has ZERO matches in the entire repo outside docs/centers.md:69 itself. STALE.
- L74-80 "What the LLM Sees" example: format matches proxy_format_ccr_output output
  shape (preview line + `[ct: metadata;center=X]` + marker) - keep.

Action: keep the page, correct/delete the two Python rows (center is wired on the Rust
tool-relay + proxy paths only in 1.4.6), keep the v2-v4 sketch labels.

---

## Discrepancy log (for orchestrator to append to DOC-REWRITE-PUBLIC-2026-09-19.md)

- [2026-09-19] docs/centers.md:68-69: claimed Python `_ccr_marker` center param and
  `X-Aphrodite-Center` header; neither exists in plugins/aphrodite/ source - center is
  Rust-only (crates/aphrodite-hermes/src/tools.rs:305, schemas.rs:84; crates/aphrodite/src/proxy.rs:1915-1920).
- [2026-09-19] docs/hermes-tool-output-schemas.md:5: links docs/ccr/content-types.md,
  which does not exist post-rewrite; taxonomy lives at docs/classification/content-types.md.
- [2026-09-19] docs/hermes-tool-output-schemas.md:5,22: tree/Development links; default
  branch is Current (remotes/Source/HEAD -> Source/Current).
- [2026-09-19] docs/hermes-tool-output-schemas.md:10,27: "22 classification types" stale;
  docs/classification/content-types.md carries 30 content types / 41 preview-shape rows.