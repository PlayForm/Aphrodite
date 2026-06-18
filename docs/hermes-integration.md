# Hermes Integration

How Aphrodite connects to Hermes Agent - and why it's different from a plain proxy.

## Architecture

```
Hermes Agent
  │
  ├── on_session_start → launch proxy (:9798)
  ├── transform_tool_result → compress tool output
  ├── transform_terminal_output → compress shell output
  ├── pre_llm_call → inject CCR catalog + retrieval hints
  ├── post_llm_call → track compression metrics
  └── context_engine → offload old messages to CCR
```

All five hooks register in `plugin.yaml`. Zero changes to Hermes core.

## Hooks Detail

| Hook | When it fires | What Aphrodite does |
|------|--------------|-------------------|
| `on_session_start` | Once, at session init | Starts cache proxy (:9797) + token proxy (:9798) as subprocesses |
| `transform_tool_result` | Every tool call returns | Classify output → compress → replace with CCR preview marker |
| `transform_terminal_output` | Every terminal command finishes | Catch stdout/stderr → compress if above threshold |
| `pre_llm_call` | Before each API call | Append CCR catalog summary + "use aphrodite_retrieve(hash)" hint |
| `post_llm_call` | After each API response | Record compression stats, update EMA, track tokens saved |

## Proxy vs Plugin

| | Native Hermes Plugin | Generic Proxy (any client) |
|---|---|---|
| How it works | Hook registration in `plugin.yaml` | Point `base_url` at `:9798` |
| Tool output compression | ✅ `transform_tool_result` intercepts | ❌ Proxy only sees HTTP traffic |
| Terminal compression | ✅ `transform_terminal_output` | ❌ |
| Context engine | ✅ Compresses middle messages | ❌ |
| Auto-launch | ✅ Proxies start automatically | ❌ Manual `aphrodite` command |
| aphrodite_* tools | ✅ 12 tools in agent namespace | ❌ Agent doesn't know about them |
| Bundled skills | ✅ 9 skills auto-loaded | ❌ |
| Prompt injection | ✅ Retrieval guidance added | ❌ |
| CCR storage | ✅ Token + cache proxy | ✅ Token + cache proxy |
| Works with | Hermes only | Any OpenAI-compatible client |
| Setup | `hermes plugins enable aphrodite` | `OPENAI_BASE_URL=:9798` |

## What the Agent Sees

Without Aphrodite, the agent's context fills with raw tool output:

```
500 tokens of build output... scrolling... error? no... warning? no...
ok it passed. That was 500 tokens I'll never get back.
```

With native Hermes integration, `transform_tool_result` replaces it:

```
[build:0E 0W 1L]  ← 15 tokens. Agent knows: build passed. Next task.
```

The agent can retrieve the full output with `aphrodite_retrieve(hash)` only if
it actually needs it - but for clean builds, exit=0 terminals, and small diffs,
the preview is enough.

## Why Not Just a Proxy?

A generic proxy compresses HTTP response bodies. That helps, but:

1. **Tool output never hits the wire** - Hermes tool calls run locally.
   `transform_tool_result` intercepts the return value BEFORE it becomes part
   of the message history. A proxy can't see this.

2. **Terminal output is local** - `transform_terminal_output` catches shell
   command stdout/stderr before Hermes even processes it. The proxy has no
   visibility.

3. **Context engine needs message access** - The engine reads the conversation
   history to decide which messages to offload to CCR. A proxy sees individual
   HTTP requests, not the full context.

4. **Agent augmentation** - The 12 `aphrodite_*` tools and 9 bundled skills
   teach the agent HOW to use compression. A proxy is opaque - the agent
   doesn't know compression exists.

## Setup

```bash
# 1. Clone the standalone plugin repo
git clone https://github.com/PlayForm/Aphrodite-Hermes.git

# 2. Symlink into your Hermes profile
ln -s "$(pwd)/Aphrodite-Hermes" ~/.hermes/profiles/<profile>/plugins/aphrodite

# 3. Enable
hermes plugins enable aphrodite

# 4. Restart (plugin loads fresh)
hermes
```

The binary auto-downloads from GitHub releases on first launch. No Rust toolchain needed.
