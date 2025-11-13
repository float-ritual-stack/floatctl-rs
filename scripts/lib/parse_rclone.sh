#!/bin/bash
#
# Parse rclone output for structured statistics
#
# Usage:
#   rclone_output=$(rclone sync ... 2>&1)
#   exit_code=$?
#   parse_rclone_output "$rclone_output" "$exit_code"
#
# Returns:
#   files_transferred: integer count
#   bytes_transferred: bytes (converted from KiB/MiB/GiB to bytes)
#   duration_sec: float seconds
#   success: boolean (based on exit code and ERROR lines)
#   error_message: string (if errors detected)

# Parse rclone "Transferred:" line for file count
# Example: "Transferred:          128 / 128, 100%"
parse_files_transferred() {
    local output="$1"
    echo "$output" | grep -E '^Transferred:[[:space:]]+[0-9]+ / [0-9]+' | \
        head -1 | \
        sed -E 's/^Transferred:[[:space:]]+([0-9]+) \/.*/\1/' || echo "0"
}

# Parse rclone "Transferred:" line for bytes
# Example: "Transferred:   	    1.477 MiB / 1.477 MiB, 100%, 154.128 KiB/s, ETA 0s"
parse_bytes_transferred() {
    local output="$1"
    local bytes_line=$(echo "$output" | grep -E 'Transferred:[[:space:]]+[0-9.]+[[:space:]]+(B|KiB|MiB|GiB)' | head -1)

    if [ -z "$bytes_line" ]; then
        echo "0"
        return
    fi

    # Extract number and unit using sed
    local number=$(echo "$bytes_line" | sed -E 's/^Transferred:[[:space:]]+([0-9.]+)[[:space:]]+.*/\1/')
    local unit=$(echo "$bytes_line" | sed -E 's/^Transferred:[[:space:]]+[0-9.]+[[:space:]]+(B|KiB|MiB|GiB).*/\1/')

    # Convert to bytes
    case "$unit" in
        B)   echo "$number" | awk '{printf "%.0f", $1}' ;;
        KiB) echo "$number" | awk '{printf "%.0f", $1 * 1024}' ;;
        MiB) echo "$number" | awk '{printf "%.0f", $1 * 1024 * 1024}' ;;
        GiB) echo "$number" | awk '{printf "%.0f", $1 * 1024 * 1024 * 1024}' ;;
        *)   echo "0" ;;
    esac
}

# Parse rclone "Elapsed time:" line
# Example: "Elapsed time:         9.9s"
parse_duration() {
    local output="$1"
    echo "$output" | grep -E 'Elapsed time:' | \
        sed -E 's/^Elapsed time:[[:space:]]+([0-9.]+)s.*/\1/' || echo "0"
}

# Calculate transfer rate (bytes per second)
# Args: bytes_transferred, duration_sec
calculate_rate() {
    local bytes="$1"
    local duration="$2"

    if [ "$duration" = "0" ] || [ -z "$duration" ]; then
        echo "null"
    else
        echo "$bytes $duration" | awk '{printf "%.0f", $1 / $2}'
    fi
}

# Check for ERROR lines in output
has_errors() {
    local output="$1"
    echo "$output" | grep -q '^[0-9/]* ERROR' && echo "true" || echo "false"
}

# Extract first ERROR message
get_error_message() {
    local output="$1"
    local error_line=$(echo "$output" | grep '^[0-9/]* ERROR' | head -1)

    if [ -z "$error_line" ]; then
        echo "null"
    else
        # Remove timestamp prefix and escape for JSON
        local msg=$(echo "$error_line" | sed -E 's/^[0-9/: ]* ERROR : //')
        msg=$(echo "$msg" | sed 's/\\/\\\\/g' | sed 's/"/\\"/g' | tr '\n' ' ')
        echo "\"$msg\""
    fi
}

# Main parsing function
# Args: rclone_output_text, exit_code
parse_rclone_output() {
    local output="$1"
    local exit_code="$2"

    local files=$(parse_files_transferred "$output")
    local bytes=$(parse_bytes_transferred "$output")
    local duration=$(parse_duration "$output")
    local rate=$(calculate_rate "$bytes" "$duration")

    # Determine success: exit code 0 AND no ERROR lines
    local has_err=$(has_errors "$output")
    local success="true"
    if [ "$exit_code" -ne 0 ] || [ "$has_err" = "true" ]; then
        success="false"
    fi

    local error_msg=$(get_error_message "$output")

    # Convert duration to milliseconds (ensure duration is not empty)
    [ -z "$duration" ] && duration="0"
    local duration_ms=$(echo "$duration * 1000" | bc | awk '{printf "%.0f", $1}')

    # Output as shell variables (source this output)
    cat <<EOF
RCLONE_FILES=$files
RCLONE_BYTES=$bytes
RCLONE_DURATION_MS=$duration_ms
RCLONE_RATE_BPS=$rate
RCLONE_SUCCESS=$success
RCLONE_ERROR=$error_msg
EOF
}

# If script is executed directly (not sourced), run test
if [ "${BASH_SOURCE[0]}" = "${0}" ]; then
    # Test with sample rclone output
    test_output='2025/10/30 13:44:09 INFO  : 2025-10-30.md: Copied (replaced existing)
2025/10/30 13:44:09 INFO  :
Transferred:   	    3.035 KiB / 3.035 KiB, 100%, 0 B/s, ETA -
Checks:               143 / 143, 100%, Listed 287
Transferred:            1 / 1, 100%
Elapsed time:         1.5s'

    echo "Testing rclone parser with sample output:"
    echo "---"
    parse_rclone_output "$test_output" 0
    echo "---"
    echo "Expected: RCLONE_FILES=1, RCLONE_BYTES=3108, RCLONE_DURATION_MS=1500, RCLONE_SUCCESS=true"
fi
