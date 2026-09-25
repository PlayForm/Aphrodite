---
name: hermes-agent-skill-authoring
description: "Use when authoring or editing repo skills in the Aphrodite `.hermes/skills/` tree. Covers frontmatter conventions, validator constraints, structure, and naming."
version: 1.1.0
author: Hermes Agent
license: MIT
platforms: [linux, macos, windows]
category: curation
category_taxonomy: curation/hermes-agent-skill-authoring
date: 2026-09-25
metadata:
    hermes:
        tags: [curation, authoring, editing, validating, naming, structuring]
        related_skills:
            - skill-library-refactoring
            - hermes-agent
status: active
scope:
    repositories:
        - PlayForm/Aphrodite
    directories:
        - .hermes/skills/
owns:
    - SKILL.md authoring mechanics inside .hermes/skills/ (validator constraints, structure, naming)
    - Frontmatter conventions for repo skills (house shape, size limits)
    - Editing rules for existing repo skills (patch vs write_file)
depends_on:
    - aphrodite-orientation (preflight gate before any write)
supersedes: []
verification:
    source_of_truth:
        - .hermes/skills/aphrodite/aphrodite-orientation/SKILL.md (frontmatter house style)
        - The Hermes skill manager validator (_validate_frontmatter)
mutation_level: mutate
---

# Authoring Repo Skills (Aphrodite)

Repo skills live in `<workspace>/.hermes/skills/<name>/SKILL.md`. Each skill
directory holds `SKILL.md` plus optional `references/`, `scripts/`, and
`templates/` subdirectories. This skill owns the authoring mechanics for that
tree: frontmatter validation, structure, naming, and editing rules.

## Where a SKILL.md can live

| Location                    | Path                                            | Created with                    | Shared?                     |
| --------------------------- | ----------------------------------------------- | ------------------------------- | --------------------------- |
| User-local                  | `~/.hermes/skills/[<category>/]<name>/SKILL.md` | `skill_manage(action='create')` | No - personal               |
| In-repo (this skill's case) | `<workspace>/.hermes/skills/<name>/SKILL.md`    | `write_file` + `patch`          | Yes - committed to the repo |

`skill_manage(action='create')` targets the user-local tree only; it never
writes into the repo. For repo skills use `write_file` (creation, rewrites)
and `patch` (small edits). `skill_manage(action='patch')` and
`skill_manage(action='write_file')` also work on repo skills, but `create`
does not.

## When to Use

- User asks you to add a skill "in this repo / branch / commit"
- You're committing a reusable workflow that should ship with Aphrodite
- You're editing an existing skill under `<workspace>/.hermes/skills/`

Don't use for: personal, machine-specific skills that must not be committed -
those belong in `~/.hermes/skills/` via `skill_manage`.

## Preflight gate

This skill has `mutation_level: mutate`. Before the first write, run the
orientation gate from `aphrodite-orientation` (five read-only commands) and
record the output. Never create or rewrite a skill from the wrong branch or
over unexpected dirty state; capture `HEAD` and the remote tip if an
auto-committer may be active.

## Required Frontmatter

Source of truth: the Hermes skill manager validator (`_validate_frontmatter`).
Hard requirements:

- Starts with `---` as the first bytes (no leading blank line).
- Closes with `\n---\n` before the body.
- Parses as a YAML mapping.
- `name` field present.
- `description` field present, ≤ **1024 chars** (`MAX_DESCRIPTION_LENGTH`).
- Non-empty body after the closing `---`.

### Aphrodite house shape

Match the canonical repo header (see `.hermes/skills/curation/hermes-agent-skill-authoring`):

```yaml
---
name: my-skill-name # lowercase, hyphens, ≤64 chars (MAX_NAME_LENGTH)
description: "Use when <trigger>. <one-line behavior>."
version: 1.0.0
author: Hermes Agent
license: MIT
platforms: [macos]
category: <category> # hermes|release|docs|engineering|curation|...
category_taxonomy: <category>/<name>
date: <actualization date>
metadata:
    hermes:
        tags: [<category>, <action-verbs>...]
        related_skills: [<names>] # only live skills; planning lives in the merged 'plan' skill
status: active
scope:
    repositories:
        - PlayForm/Aphrodite
owns:
    - <what this skill owns>
depends_on:
    - aphrodite-orientation (preflight gate)
supersedes: []
verification:
    source_of_truth:
        - <files holding the ground truth for this skill>
mutation_level: read-only | mutate
---
```

`version`, `platforms`, `tags`, `status`, `scope`, `owns`, `depends_on`,
`supersedes`, `verification`, and `mutation_level` are NOT enforced by the
validator, but every repo peer has them - omit them and your skill sticks out.
Set `mutation_level` honestly: `read-only` for inspection skills, `mutate` for
skills that create or edit files (like this one).

## Size Limits

- Description: ≤ 1024 chars (enforced).
- Full SKILL.md: ≤ 100,000 chars (enforced as `MAX_SKILL_CONTENT_CHARS`,
  ~36k tokens).
- Repo peers sit at **8-14k chars**. Aim for that range. Past 20k, split the
  detail into `references/*.md` and link them from SKILL.md.

## Validation matrix

| Constraint                | Gate                                | Fail symptom              | Fix                        |
| ------------------------- | ----------------------------------- | ------------------------- | -------------------------- |
| `---` at byte 0           | `content.startswith("---")`         | leading blank line or BOM | strip before writing       |
| Closing `---` before body | regex `\n---\s*\n` on `content[3:]` | frontmatter swallows body | re-add the closing fence   |
| Parses as YAML mapping    | `yaml.safe_load(...)`               | parse error               | fix quoting in frontmatter |
| `name` present            | `"name" in fm`                      | missing name              | add lowercase-hyphen name  |
| `description` present     | `"description" in fm`               | missing description       | add "Use when ..." trigger |
| Description ≤ 1024 chars  | `len(fm["description"])`            | too long                  | shorten to trigger class   |
| Body non-empty            | `content[m.end():].strip()`         | empty body                | write the body             |
| Total ≤ 100,000 chars     | `len(content)`                      | oversized                 | split into `references/`   |

## Workflow

1. **Survey peers**: `ls .hermes/skills/`; read 2-3 peer SKILL.md files to
   match tone and structure (`aphrodite-orientation` is the house-style
   reference).
2. **Run the preflight gate** (aphrodite-orientation) and record the five
   outputs.
3. **Draft** with `write_file` to `.hermes/skills/<name>/SKILL.md`.
4. **Validate locally**:

    ```python
    import yaml, re, pathlib
    content = pathlib.Path(".hermes/skills/<name>/SKILL.md").read_text()
    assert content.startswith("---")
    m = re.search(r'\n---\s*\n', content[3:])
    fm = yaml.safe_load(content[3:m.start()+3])
    assert "name" in fm and "description" in fm
    assert len(fm["description"]) <= 1024
    assert len(content) <= 100_000
    ```

5. **Leave the change in the working tree.** Repo skills are source, not
   runtime state; staging and committing belong to the Integrate phase of the
   unified lifecycle (see `aphrodite-orientation`). Do not `git add` or
   `git commit` manually unless the active workflow explicitly requires it.
6. **Note:** the current session's skill loader is cached - `skill_view` /
   `skills_list` will not see the new skill until a new session. Expected, not
   a bug.

## Directory Placement

```
<workspace>/.hermes/skills/<skill-name>/SKILL.md
<workspace>/.hermes/skills/<skill-name>/references/<file>.md
<workspace>/.hermes/skills/<skill-name>/scripts/<file>
<workspace>/.hermes/skills/<skill-name>/templates/<file>
```

The repo tree organizes skills in category subdirectories (`aphrodite/`,
`curation/`, `docs/`, `engineering/`, `git/`, `github/`, `hermes/`,
`release/`, ...) with one archived flat project skill at the root
(`benchmark-run-summary`, archived; successor `aphrodite-benchmarking`).
Confirm the live layout with
`ls .hermes/skills/` before placing a new skill. Pick the closest existing
category; don't invent new top-level categories casually.

## Cross-Referencing Other Skills

Reference peer skills with `depends_on` in the frontmatter (repo convention)
or `metadata.hermes.related_skills` (Hermes loader convention). Both trees
(`<workspace>/.hermes/skills/` and `~/.hermes/skills/`) are visible to your
session at load time - but a user-local skill referenced from a repo skill
won't resolve for anyone who clones the repo fresh. Prefer in-repo references
only. If a frequently-referenced skill lives only in `~/.hermes/skills/`,
promote it into the repo first.

## Editing Existing Repo Skills

- **Small fix** (typo, added pitfall, tightened trigger):
  `skill_manage(action='patch', name=..., old_string=..., new_string=...)`
  works on repo skills.
- **Major rewrite:** `write_file` the whole SKILL.md.
- **Supporting files:** `write_file` into `references/`, `templates/`, or
  `scripts/`. `skill_manage(action='write_file')` also works and enforces the
  `references/templates/scripts/assets` subdir allowlist.
- **Anonymization gate:** repo skills are public. Before writing, scan the
  content for personal paths, usernames, and machine-specific tokens; scrub
  them (stand-ins: `<workspace>` for dev roots, `~` for home, neutral
  descriptors for private tooling). A skill that ships personal paths leaks
  them to every clone.
- Run the preflight gate before any edit; leave edits unstaged in the working
  tree.

## Common Pitfalls

1. **Using `skill_manage(action='create')` for a repo skill.** It writes to
   `~/.hermes/skills/`, not the repo tree. Use `write_file` for repo creation.

2. **Leading whitespace before `---`.** The validator checks
   `content.startswith("---")`; any leading blank line or BOM fails validation.

3. **Description too generic.** Peer descriptions start with "Use when ..."
   and name the trigger class, not the single task. "Use when debugging X" >
   "Debug X".

4. **Skipping the house block.** `scope/owns/depends_on/verification/
mutation_level` are not validator-enforced, but every repo peer has them;
   omitting makes the skill look half-finished. Set `mutation_level` honestly.

5. **Writing a skill that duplicates a peer.** Before creating, list the tree
   and open 2-3 peers. Prefer extending an existing skill to creating a narrow
   sibling.

6. **Expecting the current session to see the new skill.** It won't. The skill
   loader initializes at session start. Verify in a fresh session.

7. **Linking to user-local skills from a repo skill.** Works for you, breaks
   for other clones. Prefer in-repo links only.

8. **Committing from the wrong phase.** Repo skill edits are source changes;
   leave them unstaged for the Integrate phase instead of `git add`-ing
   mid-workflow.

## Verification Checklist

- [ ] File is at `<workspace>/.hermes/skills/<name>/SKILL.md`, not
      `~/.hermes/skills/`
- [ ] Frontmatter starts at byte 0 with `---`, closes with `\n---\n`
- [ ] `name`, `description`, `version`, `platforms`, `tags`, `status`,
      `scope`, `owns`, `depends_on`, `verification`, `mutation_level` present
- [ ] `tags` includes `aphrodite`
- [ ] Name ≤ 64 chars, lowercase + hyphens
- [ ] Description ≤ 1024 chars and starts with "Use when ..."
- [ ] Total file ≤ 100,000 chars (aim 8-15k)
- [ ] Structure: `# Title` → `## Overview` → `## When to Use` → body →
      `## Common Pitfalls` → `## Verification Checklist`
- [ ] `depends_on` references resolve in-repo (no user-local links)
- [ ] Orientation gate run and recorded before the first write
- [ ] No personal paths, usernames, or machine tokens in the content
- [ ] Change left unstaged in the working tree; commit belongs to the
      Integrate phase
