# Project Context Files

Use when a session isn't picking up repo rules, or when you need to decide
between `.hermes.md` and `AGENTS.md` for a repository like PlayForm/Aphrodite.
Hermes injects project-level instructions into the system prompt by reading
context files from the working directory. The discovery order is **first match
wins** - only one project context source is loaded per session.

| File (in priority order)               | Discovery                                                 | Use when                                                                                    |
| -------------------------------------- | --------------------------------------------------------- | ------------------------------------------------------------------------------------------- |
| `.hermes.md` / `HERMES.md`             | Walks parents up to the git root, stops at git root       | You want hierarchical project rules (root + per-package overrides)                          |
| `AGENTS.md` / `agents.md`              | **Cwd only** - subdirectory and parent copies are ignored | You want portable agent instructions that work the same in Hermes, Claude Code, Codex, etc. |
| `CLAUDE.md` / `claude.md`              | Cwd only                                                  | Same as AGENTS.md, Claude-flavored                                                          |
| `.cursorrules` / `.cursor/rules/*.mdc` | Cwd only                                                  | Migrating from Cursor                                                                       |

`SOUL.md` (in `$HERMES_HOME`) is independent and always loaded when present -
it sets the agent's identity, not project rules.

## Pick the right one

- **Use `.hermes.md`** when you want Hermes-specific behavior that lives above
  the cwd (root + subtree), or when you want rules to inherit from a parent
  directory. The parent walk stops at the git root, so a home-level
  `.hermes.md` won't leak into every project (a git repo's root is the
  boundary).
- **Use `AGENTS.md`** when the same project will also be worked on by other
  agents (Codex, Claude Code, OpenCode). Those tools all have their own
  conventions for `AGENTS.md`, and the "cwd only" contract keeps the file
  portable.
- **Don't put project rules in `~/.hermes/AGENTS.md`** (or any other home-level
  location). When Hermes runs with that directory as cwd, the file loads - but
  only for that one directory. For cross-project context, use `SOUL.md` (in
  `$HERMES_HOME`, identity-only) or install a skill via `hermes skills
install`.

The Aphrodite repo keeps its repo facts in `.hermes/AGENTS.md` and its dev
skills in `.hermes/skills/` (Development branch only). Those are read by the
skill system, not the project-context injector; a `.hermes.md` complements
them with cwd-scoped rules for the session.

## Size and truncation

Each context file is capped at 20,000 characters. Files longer than that get
**head + tail** truncated (the middle is dropped, with a `[...truncated...]`
marker). For large project rules, prefer splitting into multiple skills over
cramming one file.

## Security

All context files pass through the threat-pattern scanner before reaching the
system prompt. Patterns matching prompt injection or promptware are replaced
with a `[BLOCKED: ...]` placeholder. This means an `AGENTS.md` containing
obvious injection attempts won't reach the model - the scanner blocks the
content, not the file, so the rest of the file still loads.

## Disable for one session

`hermes --ignore-rules` skips auto-injection of all project context files
(`.hermes.md`, `AGENTS.md`, `CLAUDE.md`, `.cursorrules`) **and** `SOUL.md`
identity, plus user config, plugins, and MCP servers. Use it to isolate
whether a problem is your setup or Hermes itself.

## Example: a small `.hermes.md` for the Aphrodite repo

```markdown
# Aphrodite

Hermes: when working in this repo, follow these rules.

## Build

- Before declaring a change done, run `cargo build -p aphrodite -p aphrodite-hermes`.
- Scratch goes in `.hermes/tmp/`, never a system temp dir.

## CCR

- A `<<<CCR:hash|type|size>>>` marker is stored content - resolve it with
  `aphrodite_retrieve(hash)` before acting; never re-read the source file.
```

That file at `<workspace>/PlayForm/Aphrodite/.hermes.md` is auto-loaded when
Hermes runs in any subdirectory of the Aphrodite repo, but not when it runs in
an unrelated project.

**Stop if** a rule "doesn't load" - verify the file sits at or above the git
root; the parent walk stops at the git root.

**Recovery** - permitted: move/rename the context file to the right root.
Prohibited: duplicating the same rules into `~/.hermes/AGENTS.md` to "make
them global".

## Verification

| Claim                         | Test                                     | Pass condition                |
| ----------------------------- | ---------------------------------------- | ----------------------------- |
| First match wins              | `.hermes.md` + `AGENTS.md` in one root   | Only `.hermes.md` is injected |
| Parent walk stops at git root | Context file above git root, run in repo | Not injected                  |
| Truncation marker             | Context file > 20,000 chars              | `[...truncated...]` present   |
