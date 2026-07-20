#!/usr/bin/env bash
# ═══════════════════════════════════════════════════════════════════════════════
# Aphrodite Conversational Benchmark — Full Suite Runner
#
# Runs all 3 conversation scripts through all 4 scenarios:
#   1. BASELINE     — Direct DeepSeek, no proxy, no CCR
#   2. FULL         — Both cache + token proxies active
#   3. HERMES_PROXY — Cache proxy only (tool output compression)
#   4. PROXY_API    — Token proxy only (context window compression)
#
# After the benchmark run, generates full visualization suite:
#   - S2 context-shape maps (ASCII + PNG)
#   - Token comparison charts
#   - Per-turn timelines
#   - Compression efficiency dashboards
#   - Summary dashboard
#
# Usage:
#   ./run_all.sh                     # Full run, all scenarios, all conversations
#   ./run_all.sh --dry-run           # Validate setup only
#   ./run_all.sh --scenario baseline # Single scenario
#   ./run_all.sh --skip-benchmark    # Only visualize existing results
#   ./run_all.sh --run-id my_run     # Custom run ID
# ═══════════════════════════════════════════════════════════════════════════════

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"

# Source Aphrodite environment (API keys, etc.)
if [ -f "$HOME/.hermes/.env" ]; then
    set -a  # auto-export all variables
    source "$HOME/.hermes/.env"
    set +a
fi

# ── Parse arguments ──────────────────────────────────────────────────────────
DRY_RUN=false
SKIP_BENCHMARK=false
RUN_ID=""
SCENARIO=""
CONVERSATION=""
BENCH_ARGS=()

while [[ $# -gt 0 ]]; do
    case "$1" in
        --dry-run)
            DRY_RUN=true
            BENCH_ARGS+=("--dry-run")
            shift
            ;;
        --skip-benchmark)
            SKIP_BENCHMARK=true
            shift
            ;;
        --run-id)
            RUN_ID="$2"
            BENCH_ARGS+=("--run-id" "$2")
            shift 2
            ;;
        --scenario)
            SCENARIO="$2"
            BENCH_ARGS+=("--scenario" "$2")
            shift 2
            ;;
        --conversation)
            CONVERSATION="$2"
            BENCH_ARGS+=("--conversation" "$2")
            shift 2
            ;;
        *)
            echo "Unknown argument: $1"
            exit 1
            ;;
    esac
done

# ── Check prerequisites ──────────────────────────────────────────────────────
echo "═══════════════════════════════════════════════════════════════════════════"
echo "  Aphrodite Conversational Benchmark Suite"
echo "═══════════════════════════════════════════════════════════════════════════"
echo ""

if [ "$SKIP_BENCHMARK" = false ]; then
    # Check for Python dependencies
    echo "[setup] Checking Python dependencies..."
    if ! python3 -c "import matplotlib, numpy, requests" 2>/dev/null; then
        echo "  Installing missing Python packages..."
        pip3 install --break-system-packages matplotlib numpy requests pillow tiktoken 2>&1 | tail -3
    fi
    echo "  ✓ Python dependencies OK"

    # Check for DEEPSEEK_API_KEY
    if [ -z "${DEEPSEEK_API_KEY:-}" ]; then
        echo ""
        echo "ERROR: DEEPSEEK_API_KEY environment variable is not set."
        echo ""
        echo "  The benchmark requires a DeepSeek API key to run conversations."
        echo "  Set it and re-run:"
        echo ""
        echo "    export DEEPSEEK_API_KEY=sk-..."
        echo "    ./run_all.sh"
        echo ""
        exit 1
    fi
    echo "  ✓ DEEPSEEK_API_KEY is set"

    # Ensure aphrodite binary is built
    echo ""
    echo "[setup] Building aphrodite binary (release)..."
    cd "$REPO_ROOT"
    cargo build --release 2>&1 | tail -5
    echo "  ✓ aphrodite binary built"
fi

# ── Run benchmark ────────────────────────────────────────────────────────────
if [ "$SKIP_BENCHMARK" = false ]; then
    echo ""
    echo "═══════════════════════════════════════════════════════════════════════════"
    echo "  Running Conversational Benchmark"
    echo "═══════════════════════════════════════════════════════════════════════════"
    echo ""
    echo "  Scenarios: $([ -z "$SCENARIO" ] && echo "ALL (baseline, full, hermes_proxy, proxy_api)" || echo "$SCENARIO")"
    echo "  Conversations: $([ -z "$CONVERSATION" ] && echo "ALL (coding_task, exploration_task, debugging_task)" || echo "$CONVERSATION")"
    echo "  Model: deepseek-flash"
    echo ""

    cd "$SCRIPT_DIR"
    python3 harness.py ${BENCH_ARGS[@]+"${BENCH_ARGS[@]}"}

    # Find the latest results directory
    RESULTS_DIR=$(ls -td results/*/ 2>/dev/null | head -1)
    if [ -z "$RESULTS_DIR" ]; then
        echo "No results directory found. Benchmark may have failed."
        exit 1
    fi
    RESULTS_DIR="$(cd "$RESULTS_DIR" && pwd)"
else
    # Find latest results dir for visualization
    RESULTS_DIR=$(ls -td "$SCRIPT_DIR/results"/*/ 2>/dev/null | head -1)
    if [ -z "$RESULTS_DIR" ]; then
        echo "No existing results found to visualize."
        echo "Run without --skip-benchmark first."
        exit 1
    fi
    RESULTS_DIR="$(cd "$RESULTS_DIR" && pwd)"
fi

# ── Generate visualizations ──────────────────────────────────────────────────
echo ""
echo "═══════════════════════════════════════════════════════════════════════════"
echo "  Generating Visualizations"
echo "═══════════════════════════════════════════════════════════════════════════"
echo ""
echo "  Results dir: $RESULTS_DIR"

cd "$SCRIPT_DIR"
python3 visualize.py "$RESULTS_DIR"

# ── Print output summary ─────────────────────────────────────────────────────
echo ""
echo "═══════════════════════════════════════════════════════════════════════════"
echo "  Benchmark Complete"
echo "═══════════════════════════════════════════════════════════════════════════"
echo ""
echo "  Results:     $RESULTS_DIR"
echo "  Charts:      $RESULTS_DIR/visualizations/"
echo ""
echo "  Open the summary dashboard:"
echo "    open $RESULTS_DIR/visualizations/summary_dashboard.png"
echo ""
echo "  View ASCII context maps:"
echo "    cat $RESULTS_DIR/visualizations/context_maps/baseline.txt"
echo ""
echo "  Full turn-by-turn history:"
echo "    ls $RESULTS_DIR/*/coding_task/turns/"
echo ""
