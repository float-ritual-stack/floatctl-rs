#!/bin/bash
# Sync float.dispatch to sysops-beta R2 bucket for AutoRAG

# Prevent concurrent runs (fork bomb protection)
PIDFILE="$HOME/.floatctl/run/dispatch-sync.pid"
mkdir -p "$(dirname "$PIDFILE")"

# Check if already running
if [ -f "$PIDFILE" ]; then
  OLD_PID=$(cat "$PIDFILE")
  if kill -0 "$OLD_PID" 2>/dev/null; then
    # Another sync is running, exit silently
    exit 0
  else
    # Stale PID file, remove it
    rm -f "$PIDFILE"
  fi
fi

# Write our PID
echo $$ > "$PIDFILE"

# Clean up PID file on exit
trap "rm -f '$PIDFILE'" EXIT

# Source structured logging and parsing helpers
source "$HOME/.floatctl/lib/log_event.sh"
source "$HOME/.floatctl/lib/parse_rclone.sh"

DAEMON="dispatch"
BUCKET="sysops-beta"
DRY_RUN="${1:-}"

# Check if dry-run mode
if [ "$DRY_RUN" = "--dry-run" ]; then
  DRY_FLAG="--dry-run"
else
  DRY_FLAG=""
fi

# Log sync start (trigger = cron when run from cron)
TRIGGER="cron"
[ -n "$FLOATCTL_TRIGGER" ] && TRIGGER="$FLOATCTL_TRIGGER"
log_sync_start "$DAEMON" "$TRIGGER"

# Filter rules (processed in order - excludes MUST come first!)
FILTERS=(
  --filter '- **/node_modules/**'
  --filter '- **/.git/**'
  --filter '- **/target/**'
  --filter '- **/__pycache__/**'
  --filter '- **/.venv/**'
  --filter '- **/venv/**'
  --filter '- **/.pytest_cache/**'
  --filter '- **/dist/**'
  --filter '- **/build/**'
  --filter '- **/.next/**'
  --filter '- **/.vercel/**'
  --filter '- **/.DS_Store'
  --filter '+ *.md'
  --filter '- *'
)

START_TIME=$(date +%s)000  # seconds to milliseconds

# Capture rclone output for parsing (no color codes when capturing)
RCLONE_OUTPUT=$(/opt/homebrew/bin/rclone sync \
  ~/float-hub/float.dispatch/ \
  r2:${BUCKET}/dispatch/ \
  "${FILTERS[@]}" \
  --stats-one-line \
  $DRY_FLAG \
  2>&1)

SYNC_STATUS=$?
END_TIME=$(date +%s)000
DURATION_MS=$((END_TIME - START_TIME))

# Parse rclone stats
eval "$(parse_rclone_output "$RCLONE_OUTPUT" $SYNC_STATUS)"

# Log sync complete with stats
log_sync_complete "$DAEMON" "$RCLONE_SUCCESS" "$RCLONE_FILES" "$RCLONE_BYTES" "$DURATION_MS" "$RCLONE_RATE_BPS" "$RCLONE_ERROR"

# If there were errors, also log them separately
if [ "$RCLONE_SUCCESS" = "false" ]; then
  log_sync_error "$DAEMON" "rclone" "$RCLONE_ERROR"
fi
