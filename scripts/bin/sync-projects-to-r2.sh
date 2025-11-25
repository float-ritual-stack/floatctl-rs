#!/bin/bash
# Sync ~/.claude/projects to sysops-beta R2 bucket for AutoRAG
# Syncs markdown and json files from Claude project conversations

# Prevent concurrent runs (fork bomb protection)
PIDFILE="$HOME/.floatctl/run/projects-sync.pid"
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

DAEMON="projects"
BUCKET="sysops-beta"
PROJECTS_DIR="$HOME/.claude/projects"

# Verify source directory exists
if [ ! -d "$PROJECTS_DIR" ]; then
  log_sync_error "$DAEMON" "config" "Projects directory not found: $PROJECTS_DIR"
  exit 1
fi

# Log sync start (trigger = cron when run from cron)
TRIGGER="cron"
[ -n "$FLOATCTL_TRIGGER" ] && TRIGGER="$FLOATCTL_TRIGGER"
log_sync_start "$DAEMON" "$TRIGGER"

# Filter rules - include markdown and json, exclude build artifacts
FILTERS=(
  --filter '- **/node_modules/**'
  --filter '- **/.git/**'
  --filter '- **/target/**'
  --filter '- **/__pycache__/**'
  --filter '- **/.DS_Store'
  --filter '+ *.md'
  --filter '+ *.json'
  --filter '+ **/'
  --filter '- *'
)

START_TIME=$(date +%s)000  # seconds to milliseconds

# Find rclone binary (macOS Homebrew path as fallback)
RCLONE_BIN=$(command -v rclone || echo /opt/homebrew/bin/rclone)
if [ ! -x "$RCLONE_BIN" ]; then
  log_sync_error "$DAEMON" "config" "rclone not found"
  exit 1
fi

# Capture rclone output for parsing
RCLONE_OUTPUT=$("$RCLONE_BIN" sync \
  "$PROJECTS_DIR" \
  r2:${BUCKET}/projects/ \
  "${FILTERS[@]}" \
  --stats-one-line \
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
