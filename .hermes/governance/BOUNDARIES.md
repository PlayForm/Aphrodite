# Aphrodite Governance - BOUNDARIES.md

The shared, non-negotiable boundary rules for the Aphrodite skill system.
This file is the governance copy of the canonical owner skill
`aphrodite-boundaries` (`.hermes/skills/aphrodite-boundaries/SKILL.md`). The
skill is the owner; this file exists so governance audits can read the rules
without loading a skill. **All other skills reference the owner skill; they
never copy these rules into their own prose.**

Terminology (live, archived, owner, verify, stop, recovery, branch-owned
identity), the branch-owned identity contract, and the Git repair taxonomy
are defined in `aphrodite-boundaries`; the orientation gate is defined in
`aphrodite-orientation`.

## Universal boundaries (state)

- Never mutate a repository, plugin, release, remote, tag, or configuration
  until the relevant precondition has been verified.
- Never infer a live behavior from a skill's prose; inspect the named source
  or run the named probe when the behavior affects data safety, releases, or
  runtime correctness.
- Never continue after a failed verification merely because a later step
  might "fix it."
- Treat an empty result as a valid state only where the workflow says it is
  valid, such as an already-contained cherry-pick or an intentionally empty
  sync-back.

## Universal boundaries (git)

- A branch-owned path may be restored **only** as part of an explicitly named
  promotion or sync ritual, and only from the branch's committed baseline.
- Never use `git checkout`, `git restore`, `reset`, rebase, or force-push to
  repair accidental content changes; repair intended file content directly and
  validate it.
- A tag is immutable historical evidence: create it only after the exact
  release-sync commit and all tag prerequisites have passed.
- Work bottom-up across submodules: validate and publish the plugin commit
  before updating the parent gitlink.

## Universal boundaries (context)

- Never compress a retrieval response or an Aphrodite diagnostic response;
  doing so can turn the retrieval path into a marker-resolution loop.
- Never split a tool call from its matching tool result when selecting a
  context-compression boundary.
- Treat hook input structures as read-only unless the framework documents
  mutability; the `prellmcall` history is a copy, so in-place edits are
  discarded.
- A hook that replaces output must return the original output for pass-through;
  an empty string is a destructive replacement, not "no change."

## Universal boundaries (release)

- Never bump `BINARYVERSION` before the referenced release assets exist and
  are accessible.
- Never reuse a claimed version; check the registry before publishing a
  version that matters.
- Never claim a release is complete based on a plugin banner or Git state
  alone; use runtime probes plus artifact verification.

## Secrets model

- Verify only **presence**, source name, and redacted fingerprint-never print
  an API key.
- Precedence is defined and stable: environment overrides TOML; a credential
  store (if configured) sits below environment. State whether an empty value
  overrides a valid lower-precedence source.
- Child-process passthrough is tested without echoing secrets.
- On a missing key, emit a clear degraded-state message rather than an opaque
  proxy failure.
- Never include secrets in CCR payloads, previews, logs, benchmark fixtures,
  release notes, or issue reports.
- Rotate/revoke externally exposed secrets rather than relying on Git history
  cleanup.

## Human approval boundaries

Technical readiness ("safe to release") never authorizes an external side
effect. Each of the following pauses the workflow with an explicit
`Ready for approval:` summary (current commit, proposed tag, plugin commit,
expected triggered workflows, expected external publications, verification
gates passed, known degraded conditions):

- Commit creation
- Push
- Tag creation
- GitHub release publication
- Artifact attachment
- Workflow dispatch
- Registry publication
- Remote configuration change
- Deletion or history rewrite

## Confidence labels (used by every skill)

Compact annotation after any claim that may drift. All skills use these exact
labels:

| Label               | Meaning                                               | Required handling                           |
| ------------------- | ----------------------------------------------------- | ------------------------------------------- |
| **Invariant**       | Expected to remain true unless architecture changes   | State as a permanent safety rule            |
| **Source-derived**  | Must be checked in currently checked-out source       | Name the source; re-derive before edits     |
| **Runtime-derived** | Must be read from the active process/configuration    | Read live at probe time; never hardcode     |
| **Release-derived** | Must be checked at the exact proposed release commit  | Inspect the workflow files at that commit   |
| **Historical**      | Explanatory only; never copy into live implementation | Archive or label with time/version boundary |

Format:

> **Contract:** `<claim>` **Confidence:** `<label>` **Verify:** `<probe or
source>` **If different:** `<update action>`

A live skill never presents an implementation property as an invariant.

## Mandatory step format (all live skills)

Every operational step uses the same granular contract; a step never silently
combines mutation, validation, publishing, and recovery:

````md
### Step N - <single outcome>

**Purpose:** One sentence describing the state transition.

**Preconditions**

- Observable condition A
- Observable condition B

**Do**

```sh
<one bounded command group>
```
````

**Verify**

```sh
<read-only command>
```

**Expected**

- Concrete success signal

**Stop if**

- Condition that makes continuing unsafe

**Recovery**

- Explicitly permitted repair action
- Explicitly prohibited action

**Produces**

- Commit, file, state, or evidence created by this step

```

## Fact classes (stop treating prose as truth)

| Fact class | Example | How skills should use it |
| --- | --- | --- |
| Invariant | Never split a tool call from its tool result | State as a permanent safety rule |
| Repository contract | `.gitmodules` branch field is branch-owned | Verify at each promotion boundary |
| Implementation property | Specific hook keyword names | Re-derive from checked-out source before edits |
| Runtime property | Active threshold/port/config value | Read from running configuration at probe time |
| Historical observation | A past dependency break or old cache implementation | Archive or label with time/version boundary |
| Policy decision | Do not recreate removed hooks | State owner, rationale, and allowed exception process |

## Ownership precedence

If a governance file and a skill disagree, the skill named as canonical owner
in SKILL-MANIFEST.md wins, and the conflict must be logged in
CONTRADICTION-REGISTER.md. If this file and the owner skill disagree, the
owner skill wins; fix this file to match. Verify rule uniqueness with a grep:
a rule text appearing in two live skills is a duplication violation unless one
of them is the declared owner.
```
