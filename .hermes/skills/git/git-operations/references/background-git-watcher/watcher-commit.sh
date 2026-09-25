#!/bin/bash
# File watcher + auto-stager + periodic committer for the PlayForm/Aphrodite monorepo
# Watches the Aphrodite root (crates/, plugins/, vendor/, .githooks) for changes
# Stages files continuously, commits every ~60 seconds
set -o pipefail

# Unbuffer output
export PYTHONUNBUFFERED=1
LOG_DIR="${HOME}/.hermes/tmp"
mkdir -p "$LOG_DIR"
exec > >(tee -a "$LOG_DIR/watcher-commit.log") 2>&1

# Override with the absolute clone path at runtime; '<workspace>' is the public
# placeholder - never commit a personal path into this script.
WATCH_DIR="${WATCH_DIR:-<workspace>/PlayForm/Aphrodite}"
INTERVAL=60          # commit interval in seconds
POLL_INTERVAL=5      # git status poll interval in seconds
STAGED_SINCE=0       # timestamp of last commit
FIRST_RUN=true
FORMAT_CMD="${FORMAT_CMD:-cargo fmt --all}"

echo "[$(date +%H:%M:%S)] ===== WATCHER STARTED ====="
echo "[$(date +%H:%M:%S)] Watching: $WATCH_DIR (crates/, plugins/, vendor/, .githooks)"
echo "[$(date +%H:%M:%S)] Commit interval: ${INTERVAL}s, Poll interval: ${POLL_INTERVAL}s"
echo ""

cd "$WATCH_DIR" || exit 1

while true; do
    NOW=$(date +%s)

    # --- Check for changes ---
    CHANGES=$(git status --porcelain 2>/dev/null)

    if [ -n "$CHANGES" ]; then
        # Stage everything
        git add -A 2>/dev/null
        STAGED_COUNT=$(git diff --cached --name-only 2>/dev/null | wc -l | tr -d ' ')
        if [ "$STAGED_COUNT" -gt 0 ] && [ "$STAGED_COUNT" -ne "$PREV_STAGED" ]; then
            echo "[$(date +%H:%M:%S)] Staged ${STAGED_COUNT} file(s):"
            git diff --cached --name-only 2>/dev/null | head -15
            TOTAL=$(git diff --cached --name-only 2>/dev/null | wc -l | tr -d ' ')
            if [ "$TOTAL" -gt 15 ]; then
                echo "  ... and $((TOTAL - 15)) more"
            fi
            echo ""
            PREV_STAGED=$STAGED_COUNT
        fi
    fi

    # --- Check if it's time to commit ---
    ELAPSED=$(( NOW - STAGED_SINCE ))
    STAGED_FILES_COUNT=$(git diff --cached --name-only 2>/dev/null | wc -l | tr -d ' ')
    STAGED_FILES_COUNT=${STAGED_FILES_COUNT:-0}

    if [ "$STAGED_FILES_COUNT" -gt 0 ] && { [ "$ELAPSED" -ge "$INTERVAL" ] || [ "$FIRST_RUN" = true ]; }; then
        FIRST_RUN=false
        echo "[$(date +%H:%M:%S)] ===== COMMIT CYCLE ====="
        echo "[$(date +%H:%M:%S)] Committing ${STAGED_FILES_COUNT} staged file(s)"

        # Run the formatter first
        echo "[$(date +%H:%M:%S)] Running ${FORMAT_CMD}..."
        $FORMAT_CMD 2>&1
        FMT_EXIT=$?
        if [ "$FMT_EXIT" -eq 0 ]; then
            echo "[$(date +%H:%M:%S)] Formatter OK"
            git add -A 2>/dev/null
        else
            echo "[$(date +%H:%M:%S)] Formatter exit: $FMT_EXIT (continuing)"
        fi

        # Try git gcommit
        echo "[$(date +%H:%M:%S)] Running git gcommit..."
        git gcommit 2>&1
        COMMIT_EXIT=$?
        if [ "$COMMIT_EXIT" -eq 0 ]; then
            echo "[$(date +%H:%M:%S)] ✓ git gcommit succeeded"
            echo "[$(date +%H:%M:%S)] → $(git log --oneline -1 2>/dev/null)"
            STAGED_SINCE=$(date +%s)
            PREV_STAGED=0
        else
            echo "[$(date +%H:%M:%S)] ✗ git gcommit failed (exit: $COMMIT_EXIT)"
            echo "[$(date +%H:%M:%S)] Trying git gcommit-hermes..."
            git gcommit-hermes 2>&1
            HERMES_EXIT=$?
            if [ "$HERMES_EXIT" -eq 0 ]; then
                echo "[$(date +%H:%M:%S)] ✓ git gcommit-hermes succeeded"
                echo "[$(date +%H:%M:%S)] → $(git log --oneline -1 2>/dev/null)"
                STAGED_SINCE=$(date +%s)
                PREV_STAGED=0
            else
                echo "[$(date +%H:%M:%S)] ✗ Both commit methods failed (hermes exit: $HERMES_EXIT)"
                STAGED_SINCE=$(( STAGED_SINCE + INTERVAL - 15 ))
            fi
        fi
        echo "[$(date +%H:%M:%S)] ===== END COMMIT CYCLE ====="
        echo ""
    fi

    # Faster poll near commit time
    if [ "$STAGED_FILES_COUNT" -gt 0 ] && [ "$ELAPSED" -ge "$(( INTERVAL - 10 ))" ]; then
        sleep 1
    else
        sleep "$POLL_INTERVAL"
    fi
done