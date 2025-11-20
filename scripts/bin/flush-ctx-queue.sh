#!/bin/bash
# Flush ctx queue to remote float-box server
# Runs as background daemon, retries on SSH failures

set -euo pipefail

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
trap "rm -f '$PIDFILE'" EXIT

echo "Starting ctx flush daemon (PID: $$)"
echo "Queue: $QUEUE"
echo "Remote: $REMOTE_HOST:$REMOTE_PATH"
echo "Interval: ${FLUSH_INTERVAL}s"

# Main loop
while true; do
  if [ -s "$QUEUE" ]; then
    # Try to flush
    if ssh "$REMOTE_HOST" "cat >> $REMOTE_PATH" < "$QUEUE" 2>/dev/null; then
      > "$QUEUE"  # Clear queue on success
      echo "$(date '+%Y-%m-%d %H:%M:%S') - Flushed queue to remote"
    else
      echo "$(date '+%Y-%m-%d %H:%M:%S') - SSH failed, will retry in ${FLUSH_INTERVAL}s"
    fi
  fi
  sleep "$FLUSH_INTERVAL"
done
