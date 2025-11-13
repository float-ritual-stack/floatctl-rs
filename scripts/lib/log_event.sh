#!/bin/bash
#
# Structured JSONL event logging for R2 sync daemons
#
# Usage:
#   source ~/.floatctl/lib/log_event.sh
#   log_daemon_start "daily" $$
#   log_file_change "daily" "/path/to/file" 300000
#   log_sync_start "daily" "auto"
#   log_sync_complete "daily" true 42 1024000 5000
#   log_sync_error "daily" "network" "connection timeout"
#
# Events are appended to ~/.floatctl/logs/${daemon}.jsonl
# Format: One JSON object per line (JSONL/ndjson)

LOG_DIR="$HOME/.floatctl/logs"

# Ensure log directory exists
mkdir -p "$LOG_DIR"

# Get current UTC timestamp in ISO 8601 format
_timestamp() {
    date -u +"%Y-%m-%dT%H:%M:%SZ"
}

# Append JSON line to daemon log file
_write_event() {
    local daemon="$1"
    local json="$2"
    echo "$json" >> "$LOG_DIR/${daemon}.jsonl"
}

# Log daemon start event
# Args: daemon_name, pid, [config_key=value ...]
log_daemon_start() {
    local daemon="$1"
    local pid="$2"
    shift 2

    local config_json="null"
    if [ $# -gt 0 ]; then
        # Build config object from remaining args (key=value pairs)
        local config_pairs=""
        for pair in "$@"; do
            local key="${pair%%=*}"
            local value="${pair#*=}"
            [ -n "$config_pairs" ] && config_pairs+=","
            config_pairs+="\"$key\":\"$value\""
        done
        config_json="{$config_pairs}"
    fi

    local json=$(cat <<EOF
{"event":"daemon_start","timestamp":"$(_timestamp)","daemon":"$daemon","pid":$pid,"config":$config_json}
EOF
    )
    _write_event "$daemon" "$json"
}

# Log daemon stop event
# Args: daemon_name, reason
log_daemon_stop() {
    local daemon="$1"
    local reason="$2"

    local json=$(cat <<EOF
{"event":"daemon_stop","timestamp":"$(_timestamp)","daemon":"$daemon","reason":"$reason"}
EOF
    )
    _write_event "$daemon" "$json"
}

# Log file change detected
# Args: daemon_name, file_path, debounce_ms
log_file_change() {
    local daemon="$1"
    local path="$2"
    local debounce_ms="$3"

    # Escape file path for JSON
    local escaped_path=$(echo "$path" | sed 's/\\/\\\\/g' | sed 's/"/\\"/g')

    local json=$(cat <<EOF
{"event":"file_change","timestamp":"$(_timestamp)","daemon":"$daemon","path":"$escaped_path","debounce_ms":$debounce_ms}
EOF
    )
    _write_event "$daemon" "$json"
}

# Log sync start
# Args: daemon_name, trigger ("auto"|"manual"|"cron")
log_sync_start() {
    local daemon="$1"
    local trigger="$2"

    local json=$(cat <<EOF
{"event":"sync_start","timestamp":"$(_timestamp)","daemon":"$daemon","trigger":"$trigger"}
EOF
    )
    _write_event "$daemon" "$json"
}

# Log sync complete
# Args: daemon_name, success (true|false), files_transferred, bytes_transferred, duration_ms, [transfer_rate_bps], [error_message]
log_sync_complete() {
    local daemon="$1"
    local success="$2"
    local files="${3:-0}"
    local bytes="${4:-0}"
    local duration_ms="${5:-0}"
    local rate_bps="${6:-null}"
    local error_msg="${7:-}"

    # Ensure numeric values are not empty
    [ -z "$files" ] && files="0"
    [ -z "$bytes" ] && bytes="0"
    [ -z "$duration_ms" ] && duration_ms="0"
    [ -z "$rate_bps" ] && rate_bps="null"

    local error_json="null"
    if [ -n "$error_msg" ] && [ "$error_msg" != "null" ]; then
        local escaped_error=$(echo "$error_msg" | sed 's/\\/\\\\/g' | sed 's/"/\\"/g' | tr '\n' ' ')
        error_json="\"$escaped_error\""
    fi

    local json=$(cat <<EOF
{"event":"sync_complete","timestamp":"$(_timestamp)","daemon":"$daemon","success":$success,"files_transferred":$files,"bytes_transferred":$bytes,"duration_ms":$duration_ms,"transfer_rate_bps":$rate_bps,"error_message":$error_json}
EOF
    )
    _write_event "$daemon" "$json"
}

# Log sync error
# Args: daemon_name, error_type, error_message, [context_key=value ...]
log_sync_error() {
    local daemon="$1"
    local error_type="$2"
    local error_message="$3"
    shift 3

    # Escape error message for JSON
    local escaped_msg=$(echo "$error_message" | sed 's/\\/\\\\/g' | sed 's/"/\\"/g' | tr '\n' ' ')

    local context_json="null"
    if [ $# -gt 0 ]; then
        # Build context object from remaining args
        local context_pairs=""
        for pair in "$@"; do
            local key="${pair%%=*}"
            local value="${pair#*=}"
            # Escape value for JSON
            local escaped_value=$(echo "$value" | sed 's/\\/\\\\/g' | sed 's/"/\\"/g')
            [ -n "$context_pairs" ] && context_pairs+=","
            context_pairs+="\"$key\":\"$escaped_value\""
        done
        context_json="{$context_pairs}"
    fi

    local json=$(cat <<EOF
{"event":"sync_error","timestamp":"$(_timestamp)","daemon":"$daemon","error_type":"$error_type","error_message":"$escaped_msg","context":$context_json}
EOF
    )
    _write_event "$daemon" "$json"
}
