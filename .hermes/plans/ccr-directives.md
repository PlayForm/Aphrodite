# CCR Directive System - LLM-Controlled Compression Pipeline

Status: design-only (not implemented)

## Problem

Current architecture mixes the LLM's request, tool output processing, and
compression into one opaque pipeline. The LLM has no control over what it sees -
it gets whatever the fixed pipeline produces.

## Solution

A **CCR directive system** embedded in markers that allows the LLM to pass a
small instruction set controlling how content is compressed, previewed, and
retrieved. The LLM becomes the pipeline controller.

## Architecture

```
LLM tool call
  ├── params: {file: "proxy.rs", directives: {extract: ["fn","struct"], depth: 3}}
  │
  ▼
Tool execution → raw output
  │
  ▼
Content classification (can be overridden by LLM hint)
  │
  ▼
Structure extraction (controlled by directives)
  │
  ▼
CCR storage → marker with embedded directives
  │
  ▼
Future retrieval → directives applied to output format
```

## Directive Format

Directives are JSON passed via tool call parameters or embedded in the marker's
metadata segment:

### In tool calls (primary - LLM sets intent)

```json
{
	"tool": "read_file",
	"params": {
		"path": "crates/aphrodite/src/proxy.rs",
		"_ccr": {
			"hint": "code_rust",
			"extract": ["fn", "struct", "impl", "trait"],
			"preview_depth": 5,
			"template": "full",
			"filter": "pub"
		}
	}
}
```

### In markers (persisted - survives compression)

```
<<<CCR:hash|code_rust|4832|d=fn,struct,impl;pv=5;fl=pub>>
```

## Granular Pipeline Phases

Each phase is independently controllable:

| Phase        | What                              | Directive                         |
| ------------ | --------------------------------- | --------------------------------- |
| 1. Type hint | Override auto-detection           | `hint=code_rust`                  |
| 2. Extract   | Which structural elements         | `extract=fn,struct,impl`          |
| 3. Preview   | How many lines/chars to show      | `preview_depth=5`                 |
| 4. Filter    | Content filter before compression | `filter=pub fn`                   |
| 5. Template  | Output layout                     | `template=full\|compact\|minimal` |
| 6. Retrieve  | What to return on retrieval       | `format=structure\|preview\|full` |

## Implementation Plan

### Phase 1: Directive parsing (src/directives.rs)

- Parse `_ccr` block from tool call params
- Parse embedded directives from marker metadata
- Validate against allowed directive set

### Phase 2: Pipeline hooks

- `on_classify`: override content type detection
- `on_extract`: control structure extraction depth/filter
- `on_format`: choose output template per request

### Phase 3: Tool integration

- `aphrodite_compress` accepts `directives` param
- `aphrodite_retrieve` accepts `format` + `filter` params
- `read_file`/`search_files` hooks inject `_ccr` block

### Phase 4: LLM awareness

- System prompt teaches LLM the directive vocabulary
- LLM learns to request specific structure/preview/filter
- Self-optimizing: LLM asks for what it needs

## Example: LLM asks for code structure only

```
Tool call: aphrodite_retrieve(hash="abc123", format="structure")
Response:
  [code_rust: lang=rs;fns=build_state,run_single,proxy_handler;structs=AppState,Secret;impls=AppState;ln=1989]
```

## Example: LLM asks for filtered preview

```
Tool call: aphrodite_retrieve(hash="abc123", format="preview", filter="fn ", preview_lines=3)
Response:
  fn build_state(cli: &Cli) -> anyhow::Result<AppState> {
  fn run_single(name: String, cli: Cli, rx: watch::Receiver<bool>) -> anyhow::Result<()> {
  async fn proxy_handler(State(state): State<Arc<AppState>>, ...) -> impl IntoResponse {
```

## Directive Vocabulary (LLM learns these)

| Directive       | Values                                                          | Effect                           |
| --------------- | --------------------------------------------------------------- | -------------------------------- |
| `format`        | `full`, `structure`, `preview`, `minimal`                       | What to return                   |
| `extract`       | `fn`, `struct`, `impl`, `trait`, `class`, `import`, `decorator` | Which elements                   |
| `filter`        | string pattern                                                  | Content filter before processing |
| `preview_depth` | 1-20                                                            | How many preview lines           |
| `template`      | `full`, `compact`, `minimal`                                    | Output layout                    |
| `hint`          | content type string                                             | Override auto-detection          |
| `depth`         | 1-5                                                             | Structure extraction depth       |

## Separation of Concerns

```
INCOMING (LLM → system)          INTERNAL (system → CCR)         OUTGOING (CCR → LLM)
─────────────────────────        ────────────────────────        ─────────────────────
tool call params                 content classification          marker with directives
  path, query, directives  ──►     type, size             ──►     hash, preview, structure
                                 compression decision            retrieve(tailored)
                                   threshold, budget               format=structure
                                 structure extraction               filter=pub fn
                                   depth, filter                   preview_depth=5
```

Never mix: the LLM's request parameters with the output it receives. Each is a
cleanly separated phase with its own directive namespace.
