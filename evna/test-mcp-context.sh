#!/bin/bash
# Test ask_evna via MCP to verify context injection

cd /Users/evan/float-hub-operations/floatctl-rs/evna

# Start MCP server in background
echo "Starting MCP server..."
bun run mcp-server &
MCP_PID=$!
sleep 2

# Send MCP request via stdio (simulating Claude Desktop)
echo '
{
  "jsonrpc": "2.0",
  "id": 1,
  "method": "tools/call",
  "params": {
    "name": "ask_evna",
    "arguments": {
      "query": "What Claude Code projects can you see in your context? DO NOT use tools, only look at your system prompt.",
      "timeout_ms": 10000,
      "include_projects_context": true
    }
  }
}
' | bun run mcp-server 2>&1 | head -100

# Kill MCP server
kill $MCP_PID 2>/dev/null
