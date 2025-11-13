#!/bin/bash
# File watcher for automatic R2 sync of daily notes
# 5-minute debounce to avoid rapid-fire syncs

# Source structured logging helpers
source "$HOME/.floatctl/lib/log_event.sh"

DAEMON="daily"
DEBOUNCE_MS=300000  # 5 minutes in milliseconds
PIDFILE="$HOME/.floatctl/run/daily-sync.pid"

# Track background jobs to clean up on exit
declare -a BACKGROUND_JOBS

# Write our PID atomically (noclobber prevents race condition)
mkdir -p "$(dirname "$PIDFILE")"
set -o noclobber
if ! echo $$ > "$PIDFILE" 2>/dev/null; then
  # PID file exists, check if process is still running
  if [ -f "$PIDFILE" ]; then
    OLD_PID=$(cat "$PIDFILE")
    if kill -0 "$OLD_PID" 2>/dev/null; then
      echo "Daemon already running (PID: $OLD_PID)" >&2
      set +o noclobber
      exit 1
    else
      # Stale PID file, remove it and retry
      rm -f "$PIDFILE"
      echo $$ > "$PIDFILE"
    fi
  fi
fi
set +o noclobber

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

  # Debounce: sleep for DEBOUNCE_MS milliseconds
  local debounce_seconds=$((DEBOUNCE_MS / 1000))
  sleep $debounce_seconds

  # Trigger sync script (which will log its own events)
  "$HOME/.floatctl/bin/sync-daily-to-r2.sh"
}

# Cleanup function to kill background jobs and remove PID file
cleanup() {
  # Kill all tracked background jobs
  for pid in "${BACKGROUND_JOBS[@]}"; do
    if kill -0 "$pid" 2>/dev/null; then
      kill "$pid" 2>/dev/null
    fi
  done
  rm -f "$PIDFILE"
  log_daemon_stop "$DAEMON" "signal"
}

# Trap daemon stop signals
trap 'cleanup; exit' INT TERM EXIT

# Watch daily notes directory for .md files
# Find fswatch in PATH with fallback to common locations
FSWATCH=$(command -v fswatch || echo /opt/homebrew/bin/fswatch || echo /usr/local/bin/fswatch || echo /usr/bin/fswatch)
if [ ! -x "$FSWATCH" ]; then
  echo "Error: fswatch not found. Please install: brew install fswatch" >&2
  log_daemon_stop "$DAEMON" "fswatch_not_found"
  exit 1
fi

"$FSWATCH" -0 "$DAILY_DIR" \
  --event Created \
  --event Updated \
  --event Removed \
  --include '.*\.md$' \
  --exclude '.*' \
  | while read -d "" event; do
      handle_daily_change "$event" &
      BACKGROUND_JOBS+=($!)
    done

# If fswatch exits, clean up and log it
rm -f "$PIDFILE"
log_daemon_stop "$DAEMON" "fswatch_exit"
