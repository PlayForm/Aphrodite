# Delegate brief - refactor one 5-skill group (Agent {A|B} of pair)

You are Agent {A|B} of a 2-agent pair refactoring the Aphrodite repo's Hermes
dev skills at `{repo_root}/.hermes/skills/` (Development branch - PUBLIC).
YOU own these 5 (write them):

1. {path1}
2. {path2}
3. {path3}
4. {path4}
5. {path5}
   Partner owns: {partner paths} (read-only cross-check; DO NOT edit).

HOUSE STYLE (mandatory):

- YAML frontmatter: name (identical to the dir name), description, version,
  platforms: [macos], tags, status: active; repo-owned skills add scope,
  owns, depends_on, supersedes, verification.source_of_truth,
  mutation_level.
- description: FIRST line starts "Use when <trigger>. <one-line behavior>".
- Body: concise, `## ` headings, claim-to-test tables, explicit
  Stop-if/Recovery blocks; exact commands/tools/paths; pitfalls as
  imperative lessons ("never X because Y"); NO PR/issue numbers, dates, or
  incident narration ("lessons not logs").
- Keep content accurate - never invent commands, tool names, or crate
  paths; verify against `.hermes/AGENTS.md` / `Cargo.toml` when unsure.

ANONYMIZATION (mandatory - public branch): no usernames, personal paths,
private environment files, wrapper dirs, or other-project names may remain.
Run a regex sweep over every file you wrote and fix every hit. Neutral
stand-ins: `~/.hermes/tmp/` for scratch, `$HOME` for home, `<workspace>`
for dev roots, "the private environment file" for env sourcing files. KEEP
the repo's own public-safe names: PlayForm/Aphrodite,
PlayForm/Aphrodite-Hermes, crates/, plugins/aphrodite, vendor/headroom,
vendor/rtk, ~/.hermes/aphrodite/, ~/.hermes/profiles/dev-aphrodite/,
.hermes/skills/, .hermes/tmp/.

CCR AWARENESS: in a compressed session a read may return
`<<<CCR:hash|type|size>>>` - retrieve it with aphrodite_retrieve(hash)
before acting; never re-read the file behind a marker.

TASK:

1. Read each SKILL.md fully (retrieve markers first).
2. Restructure to house style.
3. DEDUP within your 5: merge overlapping content into the better-named
   skill, remove the loser; flag DELETE-CANDIDATE for pure subsets of
   plugin-provided skills; if overlap crosses to the partner's set, note it
   in the report (never edit their files).
4. If two of YOUR skills duplicate (e.g. aphrodite-cargo-upgrade vs
   aphrodite-release-workflow both claiming version-bump ownership), keep
   the better one, fold unique content in, delete the loser.
5. Run the anonymization sweep over every file you wrote; fix every hit.

CROSS-CHECK (verify partner): read the partner's 5 skills: valid
frontmatter? trigger-first description? duplicates of your content?
structural breakage? forbidden tokens? List findings.

METHOD: read_file + write_file (full rewrite) or patch. NEVER
sed/awk/perl -i. Preserve meaningful content; don't gut a skill to make it
short.

OUTPUT: per-skill table (skill → action: restructured/merged-into/
delete-flag), dedup decisions, cross-check findings, DELETE-CANDIDATE
flags, anonymization sweep result. End with "FILES-WRITTEN: <list>".
