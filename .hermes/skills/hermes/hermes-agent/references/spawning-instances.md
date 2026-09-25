# Spawning Additional Hermes Instances

Use when a task needs a second Hermes process as a fully independent
subprocess - separate sessions, tools, and environments - instead of an
in-session `delegate_task` call. Load this reference before spawning; it holds
the one-shot, tmux, multi-agent, and resume sequences the hub skill points to.

## When to Use This vs delegate_task

|             | `delegate_task`                       | Spawning `hermes` process |
| ----------- | ------------------------------------- | ------------------------- |
| Isolation   | Separate conversation, shared process | Fully independent process |
| Duration    | Minutes (bounded by parent loop)      | Hours/days                |
| Tool access | Subset of parent's tools              | Full tool access          |
| Interactive | No                                    | Yes (PTY mode)            |
| Use case    | Quick parallel subtasks               | Long autonomous missions  |

**Stop if** the work is a quick parallel subtask - `delegate_task` is the
lighter path, and a spawned process is overhead it does not need.

## One-Shot Mode

```
terminal(command="hermes chat -q 'Benchmark the proxy: run aphrodite_test and write the summary to ~/.hermes/tmp/ccr-roundtrip.md'", timeout=300)

# Background for long tasks:
terminal(command="hermes chat -q 'Probe the hot-reload path and save findings to ~/.hermes/tmp/reload.txt'", background=true)
```

**Stop if** the one-shot call would need interaction - `hermes chat -q` has no
PTY and cannot answer follow-ups.

## Interactive PTY Mode (via tmux)

Hermes uses prompt_toolkit, which requires a real terminal. Use tmux for
interactive spawning:

```
# Start
terminal(command="tmux new-session -d -s agent1 -x 120 -y 40 'hermes'", timeout=10)

# Wait for startup, then send a message
terminal(command="sleep 8 && tmux send-keys -t agent1 'Rebuild crates/aphrodite-hermes and verify the dylib hot-reloaded' Enter", timeout=15)

# Read output
terminal(command="sleep 20 && tmux capture-pane -t agent1 -p", timeout=5)

# Send follow-up
terminal(command="tmux send-keys -t agent1 'Run aphrodite_catalog and list the compressed entries' Enter", timeout=5)

# Exit
terminal(command="tmux send-keys -t agent1 '/exit' Enter && sleep 2 && tmux kill-session -t agent1", timeout=10)
```

**Stop if** `tmux capture-pane -t <session> -p` shows no prompt after the
startup wait, or the session stops answering.

**Recovery** - send `/exit` the same way (`tmux send-keys -t agent1 '/exit'
Enter`), wait, then `tmux kill-session -t agent1` if it stays unresponsive.

## Multi-Agent Coordination

```
# Agent A: proxy crate
terminal(command="tmux new-session -d -s backend -x 120 -y 40 'hermes -w'", timeout=10)
terminal(command="sleep 8 && tmux send-keys -t backend 'Run the CCR round-trip fixture and report failures' Enter", timeout=15)

# Agent B: bridge crate
terminal(command="tmux new-session -d -s frontend -x 120 -y 40 'hermes -w'", timeout=10)
terminal(command="sleep 8 && tmux send-keys -t frontend 'Review the hook contracts in crates/aphrodite-hermes' Enter", timeout=15)

# Check progress, relay context between them
terminal(command="tmux capture-pane -t backend -p | tail -30", timeout=5)
terminal(command="tmux send-keys -t frontend 'Here are the round-trip results from the backend agent: ...' Enter", timeout=5)
```

## Session Resume

```
# Resume most recent session
terminal(command="tmux new-session -d -s resumed 'hermes --continue'", timeout=10)

# Resume specific session
terminal(command="tmux new-session -d -s resumed 'hermes --resume 20260225_143052_a1b2c3'", timeout=10)
```

## Tips

- **Never spawn for a quick subtask** - `delegate_task` carries less overhead than a full process.
- **Never spawn an agent that edits code without `-w` (worktree mode)** - it prevents git conflicts.
- **Never skip timeouts in one-shot mode** - complex tasks can take 5-10 minutes, and a hung call blocks the parent.
- **Never use raw PTY mode for interactive spawning** - prompt_toolkit mangles `\r` vs `\n`; tmux is the working path.
- **Never use a spawned process for fire-and-forget** - `hermes chat -q` needs no PTY.
- **Never schedule recurring runs with a spawn** - the `cronjob` tool handles delivery and retry.
- **Never spawn without `--profile dev-aphrodite` when the repo plugin must be visible** - the default profile does not load it.
- **Never explain "delegate_task is capped at N" as a runtime limit** - see `delegate-task-concurrency-diagnosis.md`; three real cap paths exist in Hermes, and if none fired the model is self-limiting and rationalising it as "the runtime caps."

## Local claim-to-test matrix

| Claim                                   | Evidence source               | Test                                                    | Pass condition                      | Failure response                       |
| --------------------------------------- | ----------------------------- | ------------------------------------------------------- | ----------------------------------- | -------------------------------------- |
| tmux provides a real terminal           | `tmux capture-pane -t <s> -p` | `tmux new-session -d -s agent1 -x 120 -y 40 'hermes'`   | Prompt appears in the captured pane | Send `/exit`, then `tmux kill-session` |
| `hermes chat -q` needs no PTY           | One-shot command output       | `terminal(command="hermes chat -q '...'", timeout=300)` | Command exits within the timeout    | Raise timeout; fall back to tmux       |
| `--resume <id>` restores a conversation | Session transcript            | `tmux new-session -d -s resumed 'hermes --resume <id>'` | Conversation history is present     | Check `~/.hermes/sessions/` for the id |
