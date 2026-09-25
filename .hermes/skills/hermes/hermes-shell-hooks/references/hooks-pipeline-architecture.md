# Aphrodite Shell Hooks Pipeline - Architecture Reference

Built 4-hook pipeline for formatting, QA, and context injection, keeping
Aphrodite monorepo edits (plugin source at `plugins/aphrodite`, crates under
`crates/`) formatted and surfacing prior CCR work as context.

## Installed Hooks

| Event          | Script                 | Purpose                            | Speed            |
| -------------- | ---------------------- | ---------------------------------- | ---------------- |
| post_tool_call | normalize-dashes.sh    | Unicode dashes to ASCII hyphens    | < 0.01s          |
| post_tool_call | normalize-tabs.sh      | Literal `\t` to real tab           | < 0.01s          |
| post_tool_call | post-edit-format-qa.py | shfmt/prettier/cargo fmt per file  | < 3s format      |
| post_llm_call  | self-qa-check.py       | AI-tell check + detached QA agent  | < 0.01s          |
| pre_llm_call   | prompt-context.py      | .hermes search + context injection | < 0.1s fast scan |

## Full Pipeline

```
User Request
  -> pre_llm_call: prompt-context.py
       1. Fast keyword scan of .hermes (sessions/skills/memory)
       2. QA cache check (injects findings from prev response)
       3. Librarian agent (spawned only for complex requests)
  -> LLM generates response
  -> post_llm_call: self-qa-check.py
       1. Fast AI-tell check (< 5ms) writes to cache
       2. Detached QA agent (small flash-tier model) writes to cache
  -> Background QA runs in parallel (non-blocking)

After every write_file/patch (post_tool_call):
  1. normalize-dashes.sh: Unicode dashes to ASCII
  2. normalize-tabs.sh: literal \t to real tab (patch-tool corruption fix)
  3. post-edit-format-qa.py: Format file via shfmt/prettier/cargo fmt
     Language routing: .rs=cargo fmt (nearest Cargo.toml - resolves
     crates/aphrodite or crates/aphrodite-hermes), .sh=shfmt,
     .js/.ts/.md/.yaml/.toml=prettier, .py=black
```

## Config Example

```yaml
hooks:
    post_tool_call:
        - command: ~/.hermes/agent-hooks/normalize-dashes.sh
          matcher: "write_file|patch"
        - command: ~/.hermes/agent-hooks/normalize-tabs.sh
          matcher: "write_file|patch"
        - command: ~/.hermes/agent-hooks/post-edit-format-qa.py
          matcher: "write_file|patch"
          timeout: 30
    post_llm_call:
        - command: ~/.hermes/agent-hooks/self-qa-check.py
          timeout: 10
    pre_llm_call:
        - command: ~/.hermes/agent-hooks/prompt-context.py
          timeout: 15
hooks_auto_accept: true
```

## Two-Phase Librarian Decision Logic

Spawn librarian (hermes -z) when ALL of:

- Fast scan found >= 2 keyword matches
- Request has action word OR 4+ substantive keywords OR 2+ technical terms
- NOT already cached (30 min fast, 24 hr librarian)

## Key Patterns

- Detach background agents: subprocess.Popen(start_new_session=True)
- Prevent recursion: HERMES_ACCEPT_HOOKS=0 env var
- Allowlist needs exact path match: tilde vs full path must be consistent
- post_llm_call hooks must return in < 10s; attach long tasks via nohup/detach
- pre_llm_call can return {"context": "..."} to inject into user message
- post_tool_call with matcher gates fire only for matching tool names
- Never test hooks inside the Aphrodite repo working tree - post_tool_call
  hooks rewrite files in-place; test only under ~/.hermes/tmp/
