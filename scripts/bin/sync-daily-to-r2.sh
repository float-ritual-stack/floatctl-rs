#!/bin/bash
# R2 sync script for daily notes
# Only syncs .md files from .evans-notes/daily

# Source structured logging and parsing helpers
source "$HOME/.floatctl/lib/log_event.sh"
source "$HOME/.floatctl/lib/parse_rclone.sh"

DAEMON="daily"

# Use centralized config for paths (floatctl config export)
# Falls back to default if config not available
if command -v floatctl &> /dev/null; then
  NOTES_DIR=$(floatctl config get paths.daily_notes 2>/dev/null || echo "$HOME/.evans-notes/daily")
else
  NOTES_DIR="$HOME/.evans-notes/daily"
fi

R2_REMOTE="r2:sysops-beta"

# Determine trigger source (called from watcher = auto, otherwise = manual)
TRIGGER="manual"
if [ -n "$FLOATCTL_TRIGGER" ]; then
  TRIGGER="$FLOATCTL_TRIGGER"
elif pgrep -f "watch-and-sync.sh" > /dev/null; then
  TRIGGER="auto"
fi

# Log sync start
log_sync_start "$DAEMON" "$TRIGGER"

START_TIME=$(date +%s)000  # seconds to milliseconds

# Capture rclone output for parsing
RCLONE_OUTPUT=$(rclone sync "$NOTES_DIR" "$R2_REMOTE/daily" \
  --filter '+ *.md' \
  --filter '- *' \
  --log-level INFO \
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
