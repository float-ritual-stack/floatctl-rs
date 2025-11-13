#!/bin/bash
# System health diagnostics for floatctl/evna infrastructure
# Detects zombies, duplicate processes, memory issues, and provides cleanup

set -euo pipefail

# Color codes
RED='\033[0;31m'
YELLOW='\033[1;33m'
GREEN='\033[0;32m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

ISSUES_FOUND=0

header() {
    echo -e "${BLUE}╔════════════════════════════════════════════╗${NC}"
    echo -e "${BLUE}║${NC}     FLOATCTL HEALTH CHECK                 ${BLUE}║${NC}"
    echo -e "${BLUE}╚════════════════════════════════════════════╝${NC}"
    echo
}

check_disk() {
    echo -e "${BLUE}[DISK]${NC} Checking disk space..."
    local usage=$(df -h / | tail -1 | awk '{print $5}' | tr -d '%')
    local avail=$(df -h / | tail -1 | awk '{print $4}')

    if [ "$usage" -gt 80 ]; then
        echo -e "${RED}  ✗ Disk usage critical: ${usage}% (${avail} available)${NC}"
        ISSUES_FOUND=$((ISSUES_FOUND + 1))
    elif [ "$usage" -gt 60 ]; then
        echo -e "${YELLOW}  ⚠ Disk usage elevated: ${usage}% (${avail} available)${NC}"
    else
        echo -e "${GREEN}  ✓ Disk healthy: ${usage}% used (${avail} available)${NC}"
    fi
}

check_memory() {
    echo -e "${BLUE}[MEMORY]${NC} Checking memory..."
    local free_pages=$(vm_stat | grep "Pages free" | awk '{print $3}' | tr -d '.')

    if [ "$free_pages" -lt 10000 ]; then
        echo -e "${RED}  ✗ Memory low: ${free_pages} pages free${NC}"
        ISSUES_FOUND=$((ISSUES_FOUND + 1))
    else
        echo -e "${GREEN}  ✓ Memory sufficient: ${free_pages} pages free${NC}"
    fi
}

check_zombies() {
    echo -e "${BLUE}[ZOMBIES]${NC} Checking for defunct processes..."
    local zombie_count=$(ps aux | grep -i defunct | grep -v grep | wc -l | tr -d ' ')

    if [ "$zombie_count" -gt 10 ]; then
        echo -e "${RED}  ✗ High zombie count: ${zombie_count} defunct processes${NC}"
        echo -e "${YELLOW}     Showing oldest zombies:${NC}"
        ps aux | grep -i defunct | grep -v grep | head -3 | awk '{print "     PID:", $2, "Age:", $9, "Parent:", $3}'
        echo -e "${YELLOW}     Cleanup: These will clear when parent processes are restarted${NC}"
        ISSUES_FOUND=$((ISSUES_FOUND + 1))
    elif [ "$zombie_count" -gt 0 ]; then
        echo -e "${YELLOW}  ⚠ Zombies present: ${zombie_count} defunct processes${NC}"
    else
        echo -e "${GREEN}  ✓ No zombie processes${NC}"
    fi
}

check_watch_and_sync() {
    echo -e "${BLUE}[WATCH-AND-SYNC]${NC} Checking file watcher daemon..."
    local watch_count=$(ps aux | grep "watch-and-sync.sh" | grep -v grep | wc -l | tr -d ' ')
    local main_process=$(ps aux | grep "watch-and-sync.sh" | grep -v grep | awk 'NR==1{print $2}')

    if [ "$watch_count" -eq 0 ]; then
        echo -e "${YELLOW}  ⚠ Daemon not running${NC}"
        echo -e "${YELLOW}     Start with: floatctl sync start --daemon daily${NC}"
    elif [ "$watch_count" -gt 3 ]; then
        echo -e "${RED}  ✗ Too many watch-and-sync processes: ${watch_count}${NC}"
        echo -e "${YELLOW}     Cleanup: floatctl sync stop --daemon daily && floatctl sync start --daemon daily${NC}"
        ISSUES_FOUND=$((ISSUES_FOUND + 1))
    else
        echo -e "${GREEN}  ✓ Daemon healthy: ${watch_count} processes (1 parent + background jobs)${NC}"
        if [ -f ~/.floatctl/run/daily-sync.pid ]; then
            local pidfile_pid=$(cat ~/.floatctl/run/daily-sync.pid)
            echo -e "${GREEN}     PID file: ${pidfile_pid}${NC}"
        fi
    fi
}

check_mcp_servers() {
    echo -e "${BLUE}[MCP SERVERS]${NC} Checking evna MCP servers..."
    local real_servers=$(ps auxww | grep "bun run src/mcp-server.ts" | grep -v grep | wc -l | tr -d ' ')
    local wrapper_procs=$(ps auxww | grep "bun run.*mcp-server" | grep -v "src/mcp-server.ts" | grep -v grep | wc -l | tr -d ' ')

    if [ "$real_servers" -gt 2 ]; then
        echo -e "${RED}  ✗ Duplicate MCP servers: ${real_servers} instances running${NC}"
        echo -e "${YELLOW}     Expected: 1-2 (internal + external)${NC}"
        echo -e "${YELLOW}     PIDs:${NC}"
        ps auxww | grep "bun run src/mcp-server.ts" | grep -v grep | awk '{print "     -", $2, "started", $9, "CPU:", $3"%"}'
        echo -e "${YELLOW}     Cleanup: Kill extra processes with: kill <pid>${NC}"
        ISSUES_FOUND=$((ISSUES_FOUND + 1))
    elif [ "$real_servers" -eq 0 ]; then
        echo -e "${YELLOW}  ⚠ No MCP servers running${NC}"
    else
        echo -e "${GREEN}  ✓ MCP servers healthy: ${real_servers} active${NC}"
    fi

    if [ "$wrapper_procs" -gt 3 ]; then
        echo -e "${YELLOW}  ⚠ Stale wrapper processes: ${wrapper_procs}${NC}"
        echo -e "${YELLOW}     These are usually harmless but indicate incomplete startups${NC}"
    fi
}

check_evna_remote() {
    echo -e "${BLUE}[EVNA REMOTE]${NC} Checking floatctl evna remote..."
    local remote_count=$(ps aux | grep "floatctl evna remote" | grep -v grep | wc -l | tr -d ' ')

    if [ "$remote_count" -gt 1 ]; then
        echo -e "${YELLOW}  ⚠ Multiple evna remote sessions: ${remote_count}${NC}"
        ps aux | grep "floatctl evna remote" | grep -v grep | awk '{print "     PID:", $2, "Started:", $9, "Age:", $10}'
        echo -e "${YELLOW}     Cleanup: Kill older sessions if not needed${NC}"
    elif [ "$remote_count" -eq 1 ]; then
        echo -e "${GREEN}  ✓ Evna remote running: 1 instance${NC}"
    else
        echo -e "  • No evna remote sessions (normal if not in use)"
    fi
}

check_node_processes() {
    echo -e "${BLUE}[NODE]${NC} Checking Node.js processes..."
    local node_count=$(ps aux | grep -E "node|bun" | grep -v grep | grep -v "Library/Application Support" | wc -l | tr -d ' ')
    local heavy_procs=$(ps aux | grep -E "node|bun" | grep -v grep | awk '$3 > 5.0' | wc -l | tr -d ' ')

    if [ "$heavy_procs" -gt 0 ]; then
        echo -e "${YELLOW}  ⚠ High CPU Node processes: ${heavy_procs}${NC}"
        ps aux | grep -E "node|bun" | grep -v grep | awk '$3 > 5.0' | head -3 | awk '{print "     PID:", $2, "CPU:", $3"%, Cmd:", substr($0, index($0,$11))}'
    else
        echo -e "${GREEN}  ✓ Node processes normal: ${node_count} total, none heavy${NC}"
    fi
}

check_docker() {
    echo -e "${BLUE}[DOCKER]${NC} Checking Docker..."
    if docker ps &>/dev/null; then
        local container_count=$(docker ps -q | wc -l | tr -d ' ')
        echo -e "${GREEN}  ✓ Docker running: ${container_count} containers${NC}"
    else
        echo -e "  • Docker not running (normal if not needed)"
    fi
}

summary() {
    echo
    echo -e "${BLUE}╔════════════════════════════════════════════╗${NC}"
    if [ $ISSUES_FOUND -eq 0 ]; then
        echo -e "${GREEN}║  ✓ ALL SYSTEMS HEALTHY                    ║${NC}"
    else
        echo -e "${YELLOW}║  ⚠ ${ISSUES_FOUND} ISSUE(S) DETECTED                    ║${NC}"
    fi
    echo -e "${BLUE}╚════════════════════════════════════════════╝${NC}"
}

# Main execution
header
check_disk
check_memory
check_zombies
check_watch_and_sync
check_mcp_servers
check_evna_remote
check_node_processes
check_docker
summary

exit $ISSUES_FOUND
