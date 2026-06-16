# Hermes -z Execution in Aphrodite Development

## Pattern

All aphrodite development is done via `hermes -z` flash workers. The main agent never calls tools directly.

```
Main session (pane 9, v4-pro):
  terminal(command="cd /proj && hermes -z \"<instructions>\" --model deepseek-v4-flash", background=true)
  process(action="poll", session_id="proc_...")
```

## Rules

1. Flash workers execute inline — read → patch → verify → report
2. Never launch sub-flash from flash (poll recursion)
3. Launch then do other work — never poll immediately
4. Use `--model deepseek-v4-flash` always
5. Never pass API keys in command lines
6. Never kill proxy during release
7. Analysis agents READ ONLY — no edits

## Common Prompts

### Fix bugs
```bash
hermes -z "Fix bugs X,Y,Z in file A. Read file, apply patches, verify syntax, report." --model deepseek-v4-flash
```

### Release
```bash
hermes -z "Build, test, bump, commit, push, tag, release, sync. Do NOT kill proxy." --model deepseek-v4-flash
```

### Validate
```bash
hermes -z "Test all endpoints via curl, run benchmark, write report." --model deepseek-v4-flash
```

### Research
```bash
hermes -z "Read codebase, find gaps, suggest ideas. READ ONLY." --model deepseek-v4-flash
```
