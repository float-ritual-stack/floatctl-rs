#!/bin/bash
# Flush ctx queue to remote float-box server
# Runs as background daemon, retries on SSH failures

set -euo pipefail

# Load structured logging library
source "$HOME/.floatctl/lib/log_event.sh"

DAEMON="ctx-flush"

QUEUE="$HOME/.floatctl/ctx-queue.jsonl"
PIDFILE="$HOME/.floatctl/run/ctx-flush.pid"
REMOTE_HOST="${FLOATCTL_CTX_REMOTE_HOST:-float-box}"
REMOTE_PATH="${FLOATCTL_CTX_REMOTE_PATH:-/opt/float/logs/master_stream.jsonl}"
FLUSH_INTERVAL="${FLOATCTL_CTX_FLUSH_INTERVAL:-30}"

# PID file protection (prevent duplicate daemons)
if [ -f "$PIDFILE" ]; then
  OLD_PID=$(cat "$PIDFILE")
  if kill -0 "$OLD_PID" 2>/dev/null; then
    echo "Daemon already running (PID: $OLD_PID)" >&2
    exit 0
  fi
  rm -f "$PIDFILE"
fi

# Create PID file
mkdir -p "$(dirname "$PIDFILE")"
echo $$ > "$PIDFILE"
trap 'log_daemon_stop "$DAEMON" "shutdown"; rm -f "$PIDFILE"' EXIT

# Log daemon start with configuration
log_daemon_start "$DAEMON" $$ \
  "queue=$QUEUE" \
  "remote_host=$REMOTE_HOST" \
  "remote_path=$REMOTE_PATH" \
  "flush_interval=${FLUSH_INTERVAL}s"

echo "Starting ctx flush daemon (PID: $$)"
echo "Queue: $QUEUE"
echo "Remote: $REMOTE_HOST:$REMOTE_PATH"
echo "Interval: ${FLUSH_INTERVAL}s"

# Main loop
while true; do
  if [ -s "$QUEUE" ]; then
    # Count items in queue
    QUEUE_SIZE=$(wc -l < "$QUEUE" | tr -d ' ')
    QUEUE_BYTES=$(stat -f%z "$QUEUE" 2>/dev/null || stat -c%s "$QUEUE" 2>/dev/null || echo 0)

    log_sync_start "$DAEMON" "auto"
    START_MS=$(($(date +%s) * 1000))

    # Try to flush
    if ssh "$REMOTE_HOST" "cat >> $REMOTE_PATH" < "$QUEUE" 2>/dev/null; then
      END_MS=$(($(date +%s) * 1000))
      DURATION_MS=$((END_MS - START_MS))

      > "$QUEUE"  # Clear queue on success
      log_sync_complete "$DAEMON" true "$QUEUE_SIZE" "$QUEUE_BYTES" "$DURATION_MS"
      echo "$(date '+%Y-%m-%d %H:%M:%S') - Flushed $QUEUE_SIZE items to remote"
    else
      END_MS=$(($(date +%s) * 1000))
      DURATION_MS=$((END_MS - START_MS))

      log_sync_error "$DAEMON" "ssh_failure" "Failed to connect to $REMOTE_HOST" \
        "queue_size=$QUEUE_SIZE" \
        "remote=$REMOTE_HOST:$REMOTE_PATH"
      echo "$(date '+%Y-%m-%d %H:%M:%S') - SSH failed, will retry in ${FLUSH_INTERVAL}s"
    fi
  fi
  sleep "$FLUSH_INTERVAL"
done
