# Hermes TUI Architecture for Agent Grid / Dashboard Views

## Architecture Summary

The Hermes TUI is an Ink (React) application in `ui-tui/`. It has NO plugin system for React
components - overlays are hardcoded in `appLayout.tsx` with a boolean toggle in `$overlayState`.

Aphrodite context: the agent grid is how parallel Aphrodite work is observed - subagent
delegates running crate audits, compression probes, and benchmarking sweeps - with status
glyphs, a Gantt timeline, and a footer that can surface the shell-hook pipeline status.

### Key Files

| File                                      | Purpose                                                                     |
| ----------------------------------------- | --------------------------------------------------------------------------- |
| `ui-tui/src/app/overlayStore.ts`          | Nanostore with `agents`, `approval`, `picker`, etc.                         |
| `ui-tui/src/app/interfaces.ts`            | `OverlayState` TypeScript interface                                         |
| `ui-tui/src/components/appLayout.tsx`     | Main layout. Conditional: `overlay.agents ? <AgentsOverlay> : <Transcript>` |
| `ui-tui/src/components/agentsOverlay.tsx` | Subagent dashboard (1062 lines). STATUS_GLYPH, Gantt timeline               |
| `ui-tui/src/app/turnStore.ts`             | `TurnState` with `subagents: SubagentProgress[]`                            |
| `ui-tui/src/types.ts`                     | `SubagentProgress` interface                                                |
| `ui-tui/src/app/slash/commands/ops.ts`    | Slash commands (`/agents`, `/artifacts`, etc.)                              |
| `tools/delegate_tool.py`                  | Subagent spawn, `_build_parent_callback()`, event relay (lines 719-862)     |

### Adding a New Overlay View

**Step 1: Add state to interfaces.ts**

```typescript
interface OverlayState {
	agents: boolean;
	artifacts: boolean; // NEW
	// ... existing fields
}
```

**Step 2: Add default in overlayStore.ts**

```typescript
const buildOverlayState = () => ({
	artifacts: false, // NEW
	// ...
});
// In resetFlowOverlays, also add: artifacts: $overlayState.get().artifacts,
```

**Step 3: Create component**

```typescript
// ui-tui/src/components/artifactGrid.tsx
import { Box, Text, useInput, useStdout } from '@hermes/ink'
import { useStore } from '@nanostores/react'
import { useTurnSelector } from '../app/turnStore.js'
import { patchOverlayState } from '../app/overlayStore.js'
import { $uiState } from '../app/uiStore.js'

export function ArtifactGrid() {
  const agents = useTurnSelector(s => s.subagents)
  const ui = useStore($uiState)
  const t = ui.theme
  const { stdout } = useStdout()
  const cols = stdout?.columns ?? 80

  useInput((_ch, key) => {
    if (key.escape) patchOverlayState({ artifacts: false })
  })

  return <Box>...</Box>
}
```

**Step 4: Wire into appLayout.tsx**

```typescript
import { ArtifactGrid } from './artifactGrid.js'

// Inside AppLayout render:
{overlay.agents ? (
  <AgentsOverlayPane />
) : overlay.artifacts ? (
  <ArtifactGrid />  // NEW
) : (
  <TranscriptPane ... />
)}

// Hide composer when overlay is active:
{!overlay.agents && !overlay.artifacts && ( ... )}
```

**Step 5: Add slash command in ops.ts**

```typescript
{
  aliases: ['grid'],
  help: 'open the artifact grid (live agent/dashboard view)',
  name: 'artifacts',
  run: () => {
    patchOverlayState({ artifacts: true })
  }
},
```

**Step 6: Build and restart**

```bash
cd ui-tui && npm run build
hermes gateway restart
```

### STATUS_GLYPH Reference (from agentsOverlay.tsx line 81-87)

```
running:     ● (accent color)
queued:      ○ (muted)
completed:   ✓ (statusGood / green)
interrupted: ■ (warn / yellow)
failed:      ✗ (error / red)
```

### Theme Access

- Inside components: `useStore($uiState).theme`
- Terminal columns: NOT on `Theme` - use `useStdout().stdout?.columns ?? 80`

### Key Libraries

| Library                     | Purpose                                             |
| --------------------------- | --------------------------------------------------- |
| `@hermes/ink`               | Ink React renderer (Box, Text, useInput, useStdout) |
| `@nanostores/react`         | `useStore()` for reactive state                     |
| `patchOverlayState()`       | Partial update to overlay store                     |
| `useTurnSelector(selectFn)` | Select from turn state (subagents, tools, activity) |
| `$uiState`                  | Global UI state (theme, busy, streaming, etc.)      |

### Subagent Event Flow

```
Gateway events → createGatewayEventHandler.ts → turnController.upsertSubagent()
  subagent.spawn_requested → subagent.start → subagent.thinking
    → subagent.tool → subagent.progress → subagent.complete

tools/delegate_tool.py _build_parent_callback() (lines 719-862) relays:
  subagent.start, subagent.thinking, subagent.tool, subagent.progress, subagent.complete
```

### SubagentProgress Interface (from types.ts)

```typescript
interface SubagentProgress {
	id: string;
	goal: string;
	status: "completed" | "failed" | "interrupted" | "queued" | "running";
	model?: string;
	tools: string[];
	thinking: string[];
	notes: string[];
	startedAt?: number;
	durationSeconds?: number;
	taskCount: number;
	depth: number;
	parentId: null | string;
	// tokens, cost, filesRead/Written, outputTail, etc.
}
```

### Artifact Grid Design Pattern

A responsive grid of agent cards showing:

- Traffic-light status (● RUN / ✓ DONE / ✗ FAIL)
- Role detection from goal string (librarian, formatter, worker)
- Model name (shortened), elapsed time, task count
- Activity trail (last 2 tool calls)
- Progress bar for running agents
- Footer showing hook pipeline status

Cards wrap based on terminal width: `colCount = Math.max(1, Math.floor(cols / 52))`
