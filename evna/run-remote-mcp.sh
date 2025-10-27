#!/bin/bash
# run-remote-mcp.sh - Launch evna as remote MCP server via Supergateway
#
# Usage:
#   ./run-remote-mcp.sh [--port PORT]
#
# Default port: 3100 (Supergateway SSE server)

set -euo pipefail

# Parse arguments
PORT="${1:-3100}"
if [[ "$PORT" == "--port" ]]; then
  PORT="${2:-3100}"
fi

# Navigate to evna-next directory
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
cd "$SCRIPT_DIR"

# Load environment variables
if [[ -f .env ]]; then
  set -a
  source .env
  set +a
  echo "[evna-remote] Loaded .env"
else
  echo "[evna-remote] WARNING: .env not found, using environment variables"
fi

# Ensure PATH includes cargo bin (for floatctl)
export PATH="$HOME/.cargo/bin:$HOME/.bun/bin:$PATH"
export FLOATCTL_BIN="${FLOATCTL_BIN:-$HOME/.cargo/bin/floatctl}"

# Check Supergateway installation
if ! command -v supergateway &> /dev/null; then
  echo "[evna-remote] ERROR: Supergateway not found"
  echo "[evna-remote] Install: npm install -g supergateway"
  exit 1
fi

# Check bun installation
if ! command -v bun &> /dev/null; then
  echo "[evna-remote] ERROR: bun not found"
  echo "[evna-remote] Install: curl -fsSL https://bun.sh/install | bash"
  exit 1
fi

echo "[evna-remote] Starting EVNA remote MCP server"
echo "[evna-remote] Port: $PORT"
echo "[evna-remote] Transport: stdio → SSE"
echo "[evna-remote] Access via: http://localhost:$PORT/sse"
echo "[evna-remote] Logs: stdout/stderr (redirect as needed)"
echo ""

# Run evna through Supergateway (stdio → SSE)
exec supergateway \
  --stdio "bun run --silent mcp-server" \
  --port "$PORT"
