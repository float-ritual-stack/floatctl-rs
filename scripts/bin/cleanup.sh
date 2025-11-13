#!/bin/bash
# Automated cleanup for floatctl/evna infrastructure
# Fixes duplicate processes, zombies, and stale sessions

set -euo pipefail

# Color codes
RED='\033[0;31m'
YELLOW='\033[1;33m'
GREEN='\033[0;32m'
BLUE='\033[0;34m'
NC='\033[0m'

DRY_RUN=false
FORCE=false

usage() {
    cat <<EOF
Usage: $0 [OPTIONS]

Cleanup floatctl/evna infrastructure

OPTIONS:
    --dry-run    Show what would be cleaned up without doing it
    --force      Skip confirmation prompts
    -h, --help   Show this help message

EXAMPLES:
    $0                    # Interactive cleanup
    $0 --dry-run          # Preview cleanup actions
    $0 --force            # Auto-cleanup without prompts
EOF
}

while [[ $# -gt 0 ]]; do
    case $1 in
        --dry-run)
            DRY_RUN=true
            shift
            ;;
        --force)
            FORCE=true
            shift
            ;;
        -h|--help)
            usage
            exit 0
            ;;
        *)
            echo "Unknown option: $1"
            usage
            exit 1
            ;;
    esac
done

log_action() {
    if [ "$DRY_RUN" = true ]; then
        echo -e "${BLUE}[DRY RUN]${NC} $1"
    else
        echo -e "${GREEN}[CLEANUP]${NC} $1"
    fi
}

log_skip() {
    echo -e "${YELLOW}[SKIP]${NC} $1"
}

confirm() {
    if [ "$FORCE" = true ]; then
        return 0
    fi

    local prompt="$1"
    read -p "$prompt (y/n): " -n 1 -r
    echo
    [[ $REPLY =~ ^[Yy]$ ]]
}

cleanup_duplicate_mcp() {
    echo -e "${BLUE}═══ Checking MCP Server Duplicates ═══${NC}"

    local mcp_pids=($(ps auxww | grep "bun run src/mcp-server.ts" | grep -v grep | awk '{print $2}'))
    local mcp_count=${#mcp_pids[@]}

    if [ "$mcp_count" -le 2 ]; then
        log_skip "MCP servers healthy (${mcp_count} running)"
        return 0
    fi

    echo -e "${YELLOW}Found ${mcp_count} MCP servers (expected: 1-2)${NC}"

    # Sort by start time (oldest first)
    local sorted_pids=($(ps -o pid,etime -p ${mcp_pids[@]} | tail -n +2 | sort -k2 -r | awk '{print $1}'))

    # Keep the 2 oldest (most established), kill the rest
    local to_keep=("${sorted_pids[@]:0:2}")
    local to_kill=("${sorted_pids[@]:2}")

    if [ ${#to_kill[@]} -eq 0 ]; then
        log_skip "No duplicate MCP servers to clean"
        return 0
    fi

    echo -e "${YELLOW}Will keep PIDs: ${to_keep[@]}${NC}"
    echo -e "${YELLOW}Will kill PIDs: ${to_kill[@]}${NC}"

    if confirm "Kill ${#to_kill[@]} duplicate MCP server(s)?"; then
        for pid in "${to_kill[@]}"; do
            log_action "Killing MCP server PID $pid"
            if [ "$DRY_RUN" = false ]; then
                kill -TERM "$pid" 2>/dev/null || echo "  (already gone)"
            fi
        done

        if [ "$DRY_RUN" = false ]; then
            sleep 1
            echo -e "${GREEN}✓ Cleanup complete${NC}"
        fi
    else
        log_skip "User cancelled MCP cleanup"
    fi
}

cleanup_stale_evna_remote() {
    echo -e "${BLUE}═══ Checking EVNA Remote Sessions ═══${NC}"

    local remote_pids=($(ps aux | grep "floatctl evna remote" | grep -v grep | awk '{print $2}'))
    local remote_count=${#remote_pids[@]}

    if [ "$remote_count" -le 1 ]; then
        log_skip "EVNA remote healthy (${remote_count} session)"
        return 0
    fi

    echo -e "${YELLOW}Found ${remote_count} evna remote sessions${NC}"
    ps aux | grep "floatctl evna remote" | grep -v grep | awk '{print "  PID:", $2, "Started:", $9, "Age:", $10}'

    # Sort by start time, keep newest
    local sorted_pids=($(ps -o pid,etime -p ${remote_pids[@]} | tail -n +2 | sort -k2 | awk '{print $1}'))
    local num_pids=${#sorted_pids[@]}
    local to_keep="${sorted_pids[$((num_pids-1))]}"
    local to_kill=("${sorted_pids[@]:0:$((num_pids-1))}")

    if [ ${#to_kill[@]} -eq 0 ]; then
        log_skip "No stale evna remote sessions"
        return 0
    fi

    echo -e "${YELLOW}Will keep PID: $to_keep (newest)${NC}"
    echo -e "${YELLOW}Will kill PIDs: ${to_kill[@]} (older)${NC}"

    if confirm "Kill ${#to_kill[@]} old evna remote session(s)?"; then
        for pid in "${to_kill[@]}"; do
            log_action "Killing evna remote PID $pid"
            if [ "$DRY_RUN" = false ]; then
                kill -TERM "$pid" 2>/dev/null || echo "  (already gone)"
            fi
        done

        if [ "$DRY_RUN" = false ]; then
            sleep 1
            echo -e "${GREEN}✓ Cleanup complete${NC}"
        fi
    else
        log_skip "User cancelled evna remote cleanup"
    fi
}

cleanup_zombie_processes() {
    echo -e "${BLUE}═══ Checking Zombie Processes ═══${NC}"

    local zombie_count=$(ps aux | grep -i defunct | grep -v grep | wc -l | tr -d ' ')

    if [ "$zombie_count" -eq 0 ]; then
        log_skip "No zombie processes"
        return 0
    fi

    echo -e "${YELLOW}Found ${zombie_count} zombie (defunct) processes${NC}"
    ps aux | grep -i defunct | grep -v grep | head -5 | awk '{print "  PID:", $2, "Age:", $9}'

    echo -e "${YELLOW}Note: Zombies are reaped when their parent process exits/restarts${NC}"
    echo -e "${YELLOW}      They consume no resources, just process table entries${NC}"

    log_skip "Cannot directly kill zombies (they're already dead)"
    echo -e "${YELLOW}      Recommend: Restart terminals or parent processes if count is high${NC}"
}

cleanup_wrapper_processes() {
    echo -e "${BLUE}═══ Checking Wrapper Processes ═══${NC}"

    local wrapper_count=$(ps auxww | grep "bun run.*mcp-server" | grep -v "src/mcp-server.ts" | grep -v grep | wc -l | tr -d ' ')

    if [ "$wrapper_count" -eq 0 ]; then
        log_skip "No stale wrapper processes"
        return 0
    fi

    if [ "$wrapper_count" -le 3 ]; then
        log_skip "Wrapper processes normal (${wrapper_count}, harmless)"
        return 0
    fi

    echo -e "${YELLOW}Found ${wrapper_count} wrapper processes (high)${NC}"

    local wrapper_pids=($(ps auxww | grep "bun run.*mcp-server" | grep -v "src/mcp-server.ts" | grep -v grep | awk '{print $2}'))

    if confirm "Kill ${#wrapper_pids[@]} wrapper process(es)?"; then
        for pid in "${wrapper_pids[@]}"; do
            log_action "Killing wrapper PID $pid"
            if [ "$DRY_RUN" = false ]; then
                kill -TERM "$pid" 2>/dev/null || echo "  (already gone)"
            fi
        done

        if [ "$DRY_RUN" = false ]; then
            sleep 1
            echo -e "${GREEN}✓ Cleanup complete${NC}"
        fi
    else
        log_skip "User cancelled wrapper cleanup"
    fi
}

summary() {
    echo
    echo -e "${BLUE}╔════════════════════════════════════════════╗${NC}"
    if [ "$DRY_RUN" = true ]; then
        echo -e "${BLUE}║  DRY RUN COMPLETE - No changes made      ║${NC}"
    else
        echo -e "${GREEN}║  ✓ CLEANUP COMPLETE                       ║${NC}"
    fi
    echo -e "${BLUE}╚════════════════════════════════════════════╝${NC}"

    echo
    echo "Run health check to verify:"
    echo "  ./scripts/bin/health-check.sh"
}

# Main execution
echo -e "${BLUE}╔════════════════════════════════════════════╗${NC}"
echo -e "${BLUE}║${NC}     FLOATCTL CLEANUP                      ${BLUE}║${NC}"
echo -e "${BLUE}╚════════════════════════════════════════════╝${NC}"
echo

if [ "$DRY_RUN" = true ]; then
    echo -e "${BLUE}[DRY RUN MODE - No changes will be made]${NC}"
    echo
fi

cleanup_duplicate_mcp
echo
cleanup_stale_evna_remote
echo
cleanup_zombie_processes
echo
cleanup_wrapper_processes
echo

summary
