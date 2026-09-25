---
name: benchmark-run-summary
description: "Use when summarizing an adversarial RED/BLUE/PURPLE/WHITE/BLACK benchmark run of the Aphrodite CCR engine. Reduce scattered artifacts to a scored, MEASURED-vs-SIMULATED report."
version: 1.1.0
author: curator
license: MIT
platforms: [macos, linux]
status: archived
successor: aphrodite-benchmarking
metadata:
    hermes:
        tags: [benchmark, adversarial, red-team, reporting, ste-code]
        related_skills: []
---

# Benchmark Run Summary

> **Historical (archived):** this skill documents the standalone
> `.agents/benchmark` harness, which is not repo-local; use
> `aphrodite-benchmarking` for current Aphrodite benchmark runs.

Turn the adversarial benchmark's scattered machine artifacts into one human-
readable dossier + report, with every figure labeled MEASURED or SIMULATED.
In the Aphrodite repo this summarizes the adversarial benchmark runs of the
CCR compression engine (see `aphrodite-benchmarking` for running them).

## When to Use

- A benchmark run finished and someone needs to know what it proved.
- Asked to grade a run against its declared goals, or separate measured from
  simulated evidence.
- Driving the summarizer with an agent and adding judgement on top.

## What exists on disk (the convergence surface)

The harness converges through the filesystem, not function calls. Five
participants write to three unrelated trees:

| tree                                            | written by                | holds                                          |
| ----------------------------------------------- | ------------------------- | ---------------------------------------------- |
| `.agents/benchmark/tests/<tier>-<suite>/run-*/` | `orchestrator.py`         | scored suites - the only **measured** evidence |
| `.agents/benchmark/results-control/run-*/`      | `orchestrator-control.py` | plain-assistant baseline                       |
| `<base>/variant<V>/round<N>/`                   | RED, BLUE, WHITE, BLACK   | the handshake sentinels                        |
| `<base>/{knowledge,notes,remedies,*.json}`      | WHITE + the driver        | durable memory, correspondence, verdicts       |

Raw, that is thousands of JSON files. The summarizer reduces them to a dossier
and a report - and labels every figure, because the offline pipeline produces
almost nothing but SIMULATED data.

## Run it

```bash
cd <project root>

# archive a report into .agents/benchmark/report/ (the normal daily use)
python3 .agents/benchmark/summarize_run.py

# add an LLM-written narrative (uses the hermes CLI; ~1-10 min)
python3 .agents/benchmark/summarize_run.py --llm --model tencent/hy3:free

# also echo to the terminal
python3 .agents/benchmark/summarize_run.py --stdout

# one-off: no archiving, straight to stdout
python3 .agents/benchmark/summarize_run.py --no-archive

# build the LLM prompt without calling a model
python3 .agents/benchmark/summarize_run.py --brief-out /tmp/brief.txt
```

Flags: `--base` (pipeline results base; auto-discovered), `--bench`, `--out`,
`--json`, `--no-archive`, `--stdout`, `--llm`, `--model`, `--llm-out`,
`--brief-out`, `--quiet`.

The LLM stage is optional and non-fatal: if the `hermes` CLI is missing or the
call fails, the deterministic report is still produced and archived, and the
tool says so.

## Where reports go

Everything lands under `.agents/benchmark/report/` - never the repo root.

```
.agents/benchmark/report/
├── index.json                      append-only run log
├── latest-report.md                newest deterministic report
├── latest-narrative.md             newest LLM narrative
└── YYYY-MM-DD/
    ├── <fingerprint>-report.md
    ├── <fingerprint>-dossier.json
    └── <fingerprint>-narrative.md
```

**Dedupe:** the fingerprint hashes the _evidence_ a run rests on - suites at
their paths, knowledge-base version, cycle/round counts, verdict total,
confidence trend. It contains no timestamp, so re-running over unchanged
artifacts overwrites the same three files; a genuinely new run gets a new
fingerprint. One directory per calendar day keeps daily use browsable.
`index.json` logs fingerprint, day, suites, findings count, simulated flag,
narrative presence. Add `.agents/benchmark/report/` to `.gitignore` to keep it
local.

## How the run is graded

The report grades the run against goals declared BEFORE it, in
`.agents/benchmark/GOALS.yaml` - each goal names a metric, a comparison, and a
weight:

```yaml
- id: G5
  statement: BLACK genuinely challenges the conclusion.
  check: { metric: black_dissent, op: gt, value: 0 }
  weight: primary
  blocked_by: [G3]
  rationale: A challenger that confirms everything is not testing anything.
```

Statuses: ✅ met · ❌ not met · 🚧 untestable · ❔ unknown. **🚧 untestable is the
important one** - a goal whose `blocked_by` prerequisite failed reports
untestable, not failed. Under `--skip-live`, "RED discovers escapes" and "BLACK
dissents" both become untestable, because offline they could not have been true
either way.

To change what the benchmark is trying to prove, edit `GOALS.yaml`, not the
summarizer. Available metrics are listed in the file's header comment.

## The five-colour mental model (needed to read output)

The colours are an adversarial **self-refutation** loop:

- **RED** attacks the subject and logs escapes.
- **BLUE** relocates each escaped payload to other placements and measures which
  positions still hold - separating _technique-driven_ failure from
  _placement-driven_ failure.
- **PURPLE** is a pure reader; it stitches RED × BLUE into an interplay matrix
  neither can see alone.
- **WHITE** learns content-addressed lessons, proposes remedies, validates them,
  and writes an **attack brief** handing BLACK the weakest links in everyone
  else's reasoning.
- **BLACK** attacks the _conclusion_, not the subject.

Thesis: without WHITE's brief, BLACK cannot find the soft spots and the
benchmark looks unbeatable; with it, BLACK can disprove results that only
appeared to hold. Each cycle, a claim BLACK confirms as genuinely defended is
retired (reverse-deduction note → WHITE, lesson pruned, that (technique,
placement) pair leaves the attack surface). Convergence to zero open lessons is
the intended terminal state, not a failure.

Reading the report:

- A **descending** learning curve ending at 0 means the loop reached its fixed
  point - confirm via `pruned_lessons` and `excluded_pairs`.
- A **flat** curve means the loop is not closing - usually a key mismatch
  between what `_collect_defended` emits and what `_prune_knowledge` reads.
- `pruned_lessons` and `excluded_pairs` are **cumulative per cycle** - take the
  max, never the sum.
- **All-`confirmed` from BLACK is a warning sign, not a success.** Verify BLACK
  had something real to test against.

## The three-stage analysis (`--llm`)

One call cannot both audit evidence and write well, so `--llm` runs the staged
agent chain used elsewhere in `.agents/`:

| stage         | role                                                                         | output   | sees               |
| ------------- | ---------------------------------------------------------------------------- | -------- | ------------------ |
| 1 **extract** | mine load-bearing facts, tag each MEASURED/SIMULATED, flag anomalies         | JSON     | the dossier        |
| 2 **assess**  | judge each goal against stage 1, find root causes ordered by goals unblocked | JSON     | goals + stage 1    |
| 3 **final**   | write the report                                                             | markdown | goals + stages 1-2 |

Each stage sees the previous stage's _output_, never its reasoning - stage 3
writes prose over a settled record instead of re-litigating it. Stage 2 is told
to **prefer evidence over the deterministic grade** where they disagree and mark
the goal `disputed`, which catches a metric keyed on the wrong field. Stages
degrade independently (extraction failure → chain continues on the raw dossier;
final-stage failure → deterministic report still stands). Every stage's status
is recorded in `<fingerprint>-stages.json`.

## Provenance: MEASURED vs SIMULATED

| label       | meaning                                        |
| ----------- | ---------------------------------------------- |
| `MEASURED`  | a model produced it (scored suites, live runs) |
| `SIMULATED` | a generator produced it offline                |

Under `--skip-live`, treat as simulated: the escape ledger (`_seed_escapes`),
BLUE's resistance (`_offline_resistance`), WHITE's `delta_resistance_pct`
(`simulate_validation`), and the stitch resistance BLACK checks
(`_write_stitch_reports`). CONTRACT.md §6 requires these never be presented as
measurements.

## Pitfalls (imperative lessons)

- Never present a SIMULATED number as a measurement - a provenance banner must
  state `--skip-live` before any number.
- Do not trust the deterministic grade over the evidence: when they disagree,
  the grader is the more likely to be wrong - mark the goal `disputed` and find
  the mis-keyed metric.
- `tier` is a **string** - sort numerically, not lexicographically
  (`'-1' < '-2' < '0'` corrupts the difficulty ladder).
- Exclude the `control` suite (`tier=None`, `model=mock`) from scored-tier
  counts - it is a baseline, not a scored tier.
- Treat `pruned_lessons` / `excluded_pairs` as cumulative per cycle (max, not
  sum).
- Verify BLACK had real material to test against before trusting an
  all-`confirmed` verdict - a brief compared against a driver-written constant
  confirms everything and proves nothing.
- Watch for these harness defects the tool detects: seeded rather than observed
  escape ledgers; `verification.py`'s split-half instrument never applied
  (`_challenge_remedies` reads `white['remedies_adopted']` but
  `white-done.json` stores `adopted` with no per-case outcomes); reverse
  deduction writing notes but pruning nothing (`brief_id` vs `claim_id`);
  pattern `lift` always 1.0 (`(support/total) / base_rate` where `base_rate` is
  itself `support/total`); control suite too small to support comparative
  claims; tier suites hitting the 3600s orchestrator cap; `notes/` holding only
  driver-shaped notes (the NOTES_PROTOCOL bus never ran).
- A narrative stage showing `skipped`/`unparsed` in `<fingerprint>-stages.json`
  means the narrative was written with less context than intended - say so
  rather than trusting it.

## Oneshot driver prompt

The script runs the three-stage chain itself, so a oneshot is just:

```
python3 .agents/benchmark/summarize_run.py --llm
```

Read `.agents/benchmark/report/latest-narrative.md` for the analysis and
`latest-report.md` for the deterministic evidence. To add agent judgement:

```
Read .agents/benchmark/GOALS.yaml (what this run is trying to prove) and
.agents/benchmark/CONTRACT.md §6 (provenance rules). Then run:

  python3 .agents/benchmark/summarize_run.py --llm

Read the scorecard in .agents/benchmark/report/latest-report.md and the staged
analysis in latest-narrative.md. Then tell me: which goals were met, which were
missed, and which were never testable - and for each miss, the root cause and
what unblocks it. Separate MEASURED from SIMULATED throughout; never present a
simulated number as a measurement. If the deterministic grade disagrees with the
evidence, trust the evidence and say which metric is keyed wrong.

Write nothing to the repository root; the tool already archived everything under
.agents/benchmark/report/.
```
