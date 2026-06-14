""" hermes_compress: headroom integration architecture overview
==========================================================

Four integration modes (set via compression.headroom.integration):

hook Pure hook-based - no file patching needed. transform_tool_result hook
compresses each tool output before it enters message history. Built-in
compression has less to work with. Fast, clean, restart-free config changes.

hybrid (default) Hooks + patcher. Tool outputs compressed at capture time AND
the full message list compressed before LLM API calls. Best balance of safety
and savings.

waterfall Maximum compression. Hooks compress tool outputs first, then the
patcher re-compresses the entire message list. Two-pass compression for extreme
token efficiency.

proxy External headroom server (hermes-compress proxy). Zero code in-process.
Tools speak to the proxy via headroom\_\* tools.

Hook Flow: ┌─────────────────────────────────────────────────────────┐ │ Tool
executes (terminal, read_file, execute_code, etc.) │
└──────────────────────┬──────────────────────────────────┘ │ result string
(JSON/text) ▼ ┌─────────────────────────────────────────────────────────┐ │
transform_tool_result hook ← HEADROOM COMPRESSES HERE │ │ - receives: tool_name,
args, result, tool_call_id │ │ - returns: compressed result string │ │ - safety
guard prevents empty outputs │ │ - skip strategy for vision/browser/audio tools
│ └──────────────────────┬──────────────────────────────────┘ │ compressed
result ▼ ┌─────────────────────────────────────────────────────────┐ │
make_tool_result_message() → appended to message history│
└──────────────────────┬──────────────────────────────────┘ │ ▼
┌─────────────────────────────────────────────────────────┐ │ Built-in
ContextCompressor (threshold-based summaries) │ │ Now has LESS work - tool
outputs already compressed │
└──────────────────────┬──────────────────────────────────┘ │ ▼
┌─────────────────────────────────────────────────────────┐ │ pre_llm_call
hook + patcher (hybrid/waterfall modes) │ │ Full message list compression before
API call │ └──────────────────────┬──────────────────────────────────┘ │ ▼ LLM
API Call

Hot Reload: Config is read on each hook invocation (cached for 5s). Changing
compression.headroom.\* takes effect within seconds without restart.

DEV Mode (HERMES_COMPRESS_DEV=1): read_file, terminal, execute_code, patch
downgrade to "minimal" strategy - prevents aggressive compression of dev tools
during debugging. """
