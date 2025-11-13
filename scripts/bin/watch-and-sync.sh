#!/bin/bash
# File watcher for automatic R2 sync of daily notes
# 5-minute debounce to avoid rapid-fire syncs

# Source structured logging helpers
source "$HOME/.floatctl/lib/log_event.sh"

DAEMON="daily"
DEBOUNCE_MS=300000  # 5 minutes in milliseconds
PIDFILE="$HOME/.floatctl/run/daily-sync.pid"

# Check if already running (duplicate prevention)
if [ -f "$PIDFILE" ]; then
  OLD_PID=$(cat "$PIDFILE")
  if kill -0 "$OLD_PID" 2>/dev/null; then
    echo "Daemon already running (PID: $OLD_PID)" >&2
    exit 1
  else
    # Stale PID file, remove it
    rm -f "$PIDFILE"
  fi
fi

# Write our PID
mkdir -p "$(dirname "$PIDFILE")"
echo $$ > "$PIDFILE"

# Use centralized config for paths (floatctl config export)
# Falls back to symlink resolution if config not available
if command -v floatctl &> /dev/null; then
  DAILY_DIR=$(floatctl config get paths.daily_notes 2>/dev/null)
fi

# Fallback to symlink resolution if config not available
if [ -z "$DAILY_DIR" ]; then
  DAILY_DIR=$(readlink -f "$HOME/.evans-notes/daily" 2>/dev/null || realpath "$HOME/.evans-notes/daily" 2>/dev/null || echo "$HOME/Library/Mobile Documents/com~apple~CloudDocs/.evans-notes/daily")
fi

# Log daemon start with config
log_daemon_start "$DAEMON" $$ "watch_dir=$DAILY_DIR" "debounce_ms=$DEBOUNCE_MS"

# Function to handle daily notes changes
handle_daily_change() {
  local event=$1

  # Log file change event
  log_file_change "$DAEMON" "$event" "$DEBOUNCE_MS"

  # 5 minute debounce (300 seconds)
  sleep 300

  # Trigger sync script (which will log its own events)
  "$HOME/.floatctl/bin/sync-daily-to-r2.sh"
}

# Trap daemon stop signals and clean up PID file
trap 'rm -f "$PIDFILE"; log_daemon_stop "$DAEMON" "signal"; exit' INT TERM EXIT

# Watch daily notes directory for .md files
# Use full path to fswatch to avoid PATH issues in launchd
/opt/homebrew/bin/fswatch -0 "$DAILY_DIR" \
  --event Created \
  --event Updated \
  --event Removed \
  --include '.*\.md$' \
  --exclude '.*' \
  | while read -d "" event; do
      handle_daily_change "$event" &
    done

# If fswatch exits, clean up and log it
rm -f "$PIDFILE"
log_daemon_stop "$DAEMON" "fswatch_exit"
