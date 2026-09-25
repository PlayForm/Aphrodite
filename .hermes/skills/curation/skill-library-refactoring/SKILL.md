---
name: skill-library-refactoring
description: "Use when refactoring/deduplicating the Aphrodite repo's skill library. Batch-delegation waves, house-style rewrites, governance-first ordering, public-branch anonymization gate."
version: 1.1.0
author: Hermes Agent
license: MIT
platforms: [macos]
category: curation
category_taxonomy: curation/skill-library-refactoring
date: 2026-09-25
metadata:
    hermes:
        tags: [curation, refactoring, deduplicating, wave-planning, anonymizing, governing]
        related_skills:
            - hermes-agent-skill-authoring
            - parallel-delegation-execution
status: active
scope:
    repositories:
        - PlayForm/Aphrodite
    branches:
        - Development
owns:
    - The batch refactor wave shape (5 skills per pair, 2 agents per group, 2 groups per wave)
    - The pair-brief contract (templates/pair-brief.md) and the house style it embeds
    - The governance-first ordering for the repo-owned skill set
    - The anonymization gate for the public Development branch
depends_on:
    - aphrodite-orientation (preflight gate before any mutation)
    - aphrodite-development (house style, direct-edit rule)
supersedes: []
verification:
    source_of_truth:
        - .hermes/AGENTS.md (repo facts, quality gates)
        - .hermes/governance/ (manifest and cross-reference files when present)
mutation_level: local
---

# Skill Library Refactoring

Batch refactor of the Aphrodite repo's dev skills in `.hermes/skills/`
(Development branch only - never shipped with the plugin). Triggered by
commands like "refactor every skill, deduplicate and restructure them
completely". This skill owns the refactor wave shape: it restructures
content and file layout, never runtime behavior (`mutation_level: local`).

## Preflight (Orient phase)

Run the `aphrodite-orientation` gate before the first edit:

```sh
git rev-parse --show-toplevel
git branch --show-current
git status --short
```

Expected: root is `PlayForm/Aphrodite`, branch is `Development` (repo
skills live only there - the Current branch is never a refactor target).
Capture HEAD and remote tip first (auto-committer awareness: `git status`
alone is not stable evidence).

**Stop if** - wrong root or branch, broken submodule state (see
`aphrodite-orientation` submodule diagnosis), or `.hermes/skills/` absent.

**Recovery** - re-run the gate; `git submodule update --init` only after
the parent state is verified clean. Never switch branches or clean the
working tree to "fix" the gate.

## Procedure

1. **Survey before dispatching.** List the disk set:
   `search_files(pattern='SKILL.md', target='files', path='.hermes/skills')`;
   list symlinks (`find .hermes/skills -type l`). Cross-check `skills_list`
   for the LOADED set - disk count != loaded count; many are disabled in
   config `skills.disabled`. Note duplicates: same name in two dirs (loader
   throws Ambiguous skill name) and plugin-namespaced copies
   (`aphrodite:` prefix) vs top-level copies.
2. **Group 5 skills per pair, 2 agents per group, 2 groups per wave**
   (4 concurrent children). Each agent gets DISJOINT files to write - never
   two agents writing the same SKILL.md (parallel writes clobber). The
   partner's 5 are read-only cross-check targets.
3. **Brief contract** (see `templates/pair-brief.md`): house style spec,
   dedup rules, cross-check duty, never-sed rule, anonymization mandate,
   output format. Include the house style verbatim in every brief - do not
   assume agents know it.
4. **Post-wave verification.** Spot-check every factual claim about live
   system state in rewritten skills (hooks wired?, CCR tool names, crate
   paths, runtime home layout) against the real system, using the
   AUTHORITATIVE source for that state - `hermes hooks list` for hook
   wiring, `aphrodite_stats`/`aphrodite_rebuild` for runtime health - never
   the agent's prose. Agent self-reports ("all verified") are not
   verification. Fix errors directly with patch; never append "UPDATE:"
   under the wrong text.
5. **Rate-limit recovery.** Free-tier providers kill subagents with HTTP 429
   while composing final summaries - the transcript is still complete.
   Salvage it, re-dispatch with pre-gathered evidence and a relaxed or
   absent output_schema (strict schemas caused retry failures).

## Design-spec-driven refactor of the repo-owned set

When the user hands a design spec (e.g. a review doc in the user's
Downloads folder) and says "rewrite all skills" against the repo's
`.hermes/skills/` (edited as PLAIN FILES - never skill_manage), the
refactor shape differs from the profile-skill pair flow:

1. **Every agent reads the spec file itself.** The parent hands out the
   absolute spec path in each brief instead of re-emitting 60KB+ of design
   text; agents extract their own relevant sections. The spec stays the
   single authority and the briefs stay small.
2. **Governance wave FIRST, content waves after.** One agent creates the
   canonical cross-reference layer (core skills + governance files:
   SKILL-MANIFEST.md, BOUNDARIES.md, VERIFICATION-MATRIX.md,
   ARCHIVE-INDEX.md, CONTRADICTION-REGISTER.md) before any content agent
   writes - later agents then REFERENCE the canonical rules instead of
   duplicating them ("referenced, not copied"), and their `depends_on`
   names resolve to real files.
3. **The manifest is the ownership contract, pre-declared.** The governance
   agent writes SKILL-MANIFEST.md listing the FULL target end-state -
   including skills other agents will create ("active - in flight"),
   archived skills with successors, and per-skill mutation_level - so every
   later brief can say "your ownership contract is in the manifest" and
   file ownership never needs re-negotiation mid-wave.
4. **Contradiction register is parent-merged only.** Agents REPORT
   contradictions in their completion summaries; the parent appends them to
   CONTRADICTION-REGISTER.md after the wave. Never let concurrent agents
   write the shared governance files.
5. **Archive = annotate, sync, REMOVE the live dir.** Deprecated or
   historical skills get frontmatter `status: archived` + `successor` + a
   historical cutoff note; the newer live content is copied over the
   archive copy; then the live dir is removed from `.hermes/skills/` so the
   loader stops listing them. Leave the archive dir's unrelated files
   (README, merge notes) untouched.
6. **Frontmatter contract when the loader reads the skills:** KEEP the
   loader-compatible fields (name/description/version/platforms/tags/status)
   and ADD the Aphrodite metadata (scope repositories/branches, owns,
   depends_on, supersedes, verification.source_of_truth, mutation_level).
   Dropping the original fields breaks loading; only adding is safe. The
   description's first line must start "Use when <trigger>."
7. **Verification gates:** every agent ends each rewritten skill with a
   claim-to-test matrix; the parent runs the final cross-reference audit
   (no active skill links an archived skill; all manifest "in flight" rows
   flipped to final status), the anonymization sweep (below), and fixes the
   repo's AGENTS.md skill list itself after all waves land.

## House style (embed verbatim in every brief)

- Frontmatter: `name` (identical to the dir name), `description`, `version`,
  `author: Hermes Agent`, `license: MIT`, `platforms`, `category`,
  `category_taxonomy: <category>/<name>`, `date`, `metadata.hermes.tags`
  (action verbs), `metadata.hermes.related_skills`, `status: active`;
  repo-owned skills add `scope`, `owns`, `depends_on`, `supersedes`,
  `verification.source_of_truth`, `mutation_level`.
- Description: FIRST line starts "Use when <trigger>. <one-line behavior>".
- Body: concise, `## ` headings, claim-to-test tables, explicit
  Stop-if/Recovery blocks; exact commands/tools/paths; pitfalls as
  imperative lessons ("never X because Y").
- Lessons not logs: NO PR/issue numbers, dates, version narration, or
  incident retelling.
- Keep content accurate: never invent commands, tool names, or crate paths;
  verify against `.hermes/AGENTS.md` / `Cargo.toml` when unsure.
- Skills are content-hash-scanned on load (skills_guard) - flagged content
  quarantines the skill until a re-scan passes. Keep skill templates benign
  plain content (no eval/exec patterns, no encoded payloads).

## Anonymization gate (public Development branch)

The Development branch is PUBLIC - zero personal or machine-identifying info
may remain in any touched file. Before a wave is declared done:

- Sweep every rewritten file for forbidden tokens - usernames; personal
  volume/dev-root paths; private environment file names; wrapper dirs;
  other-project monorepo names; internal process markers and queue sections -
  with a single regex search over the changed files; fix every hit.
- Neutral stand-ins: `~/.hermes/tmp/` for scratch, `$HOME`/`~` for home,
  `<workspace>` for dev roots, "the private environment file" for env
  sourcing files.
- KEEP the repo's own public-safe names: PlayForm/Aphrodite,
  PlayForm/Aphrodite-Hermes, crates/aphrodite, crates/aphrodite-hermes,
  plugins/aphrodite, vendor/headroom, vendor/rtk, ~/.hermes/aphrodite/,
  ~/.hermes/profiles/dev-aphrodite/, .hermes/skills/, .hermes/tmp/.

## Pitfalls

- **skill_manage cannot touch repo-owned skills.** A skill whose entry
  symlinks into the repo, or a trusted-project skill, reads fine via
  `skill_view` but `skill_manage` rejects every write ("Skill '<name>' not
  found in active profile"). The repo's dev skills in `.hermes/skills/`
  are edited DIRECTLY with write_file/patch - never skill_manage. When the
  user says "move all skills into the repo", the moving tool is the shell
  (`cp` + `ln -s`), not skill_manage.
- **Repo-local skills load only after `hermes skills trust <repo>`.** A
  skill merged into the repo's `.hermes/skills/` is INVISIBLE to
  `skill_view`/`skills_list` until the project is trusted:
  `hermes skills trust <repo-root>` once (trust is a security gate - every
  project SKILL.md is content-hash-scanned by the skills_guard on load, so
  a `git pull` cannot inject a malicious skill into an already-trusted
  repo). The CURRENT session's loader is cached at startup - newly trusted
  skills appear only in a NEW session.
- **CCR markers in reads are content, not noise.** In a compressed session
  a large skill read may return `<<<CCR:hash|type|size>>>`; the agent must
  `aphrodite_retrieve(hash)` before rewriting and never re-read the file
  behind a marker.
- **Classify before bulk-deleting.** Before removing a skill, grep it for
  repo-specific content (exact crate paths, hook names, repo-owned
  ceremonies) vs generic Hermes mechanics (TUI, config keys, cron, hooks).
  Generic skills serve the plugin's docs too and stay; only repo-scoped
  content goes. When the loser has unique support files (references/,
  templates/, scripts/), COPY them into the survivor's matching directory
  and add a pointer line BEFORE deleting.
- **Dedup rule:** merge overlapping content into the better-named skill,
  remove the loser; flag `DELETE-CANDIDATE` for pure subsets of
  plugin-provided skills. Both pair agents flagging the same
  DELETE-CANDIDATE independently is the signal to execute the merge at
  parent level.
- **Pacing hooks block delegate_task when the dispatch queue saturates.**
  `pre-tool-call-pace-delegate` sleeps inline; when children saturate the
  queue even `delegate_task list` times out. Stop polling and let results
  arrive as messages.
- **Cross-set overlap:** when one pair's skill overlaps the other pair's,
  resolve by cross-referencing (one points to the other) rather than
  duplicating; never edit the partner's files.

## Local claim-to-test matrix

| Claim                                 | Evidence source           | Test                                       | Pass condition                            | Failure response                    |
| ------------------------------------- | ------------------------- | ------------------------------------------ | ----------------------------------------- | ----------------------------------- |
| Wave groups write disjoint files      | Pair briefs               | Dispatch a wave; diff the assigned lists   | No two agents share a write target        | Re-group before dispatch            |
| House style reaches every agent       | `templates/pair-brief.md` | Read a dispatched brief                    | Verbatim house style + sweep rule present | Re-brief with the template          |
| Rewrites carry claim-to-test matrices | Rewritten files           | Grep for the Claim header in each file     | Every skill ends with a matrix            | Patch the matrix in                 |
| No forbidden tokens survive           | Anonymization sweep       | Regex search over changed files            | Zero hits                                 | Fix every hit before declaring done |
| CCR markers resolved before edits     | `aphrodite_retrieve`      | Read a large skill in a compressed session | Marker expanded before rewrite            | Retrieve, then edit                 |

## Files

- `templates/pair-brief.md` - the delegate_task brief skeleton for one
  5-skill group.
