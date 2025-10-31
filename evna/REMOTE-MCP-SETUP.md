# Remote MCP Setup for EVNA

This guide shows how to run EVNA as a remote MCP server accessible from any device (laptop, mobile, web) using ngrok tunneling.

## Quick Start (TL;DR)

```bash
# Install dependencies
npm install -g supergateway

# Start evna remote server (one command!)
floatctl evna remote

# Visit http://localhost:4040 to get your ngrok URL
# Add URL to Claude Desktop config and restart
```

Done! Evna is now accessible from any device.

## Architecture

```
Mac Mini (server)                     Laptop/Mobile (clients)
┌────────────────────┐                ┌──────────────────┐
│ evna (stdio)       │                │ Claude Desktop   │
│   ↓                │                │ Claude Mobile    │
│ Supergateway (SSE) │  ←── ngrok ──→ │ Claude Web       │
│   ↓                │                │                  │
│ ngrok tunnel       │                └──────────────────┘
│ (HTTPS public URL) │
└────────────────────┘
```

**Why Supergateway?** EVNA uses stdio transport (stdin/stdout), but remote clients need HTTP/SSE. Supergateway bridges stdio ↔ SSE with zero code changes.

**Why ngrok?** You already have a paid account, and it provides stable HTTPS URLs with authentication.

## Prerequisites

- [x] Mac Mini with evna installed
- [x] ngrok account (paid - you already have this)
- [ ] Supergateway (we'll install this)
- [ ] SSH access to Mac Mini (for management)

## Step 1: Install Supergateway on Mac Mini

Supergateway is a Node.js tool that converts stdio MCP servers to SSE.

```bash
# SSH into your Mac Mini
ssh your-mac-mini

# Install Supergateway (Node.js package)
npm install -g supergateway
```

## Step 2: Create EVNA SSE Server Script

Create a launcher script that runs evna through Supergateway:

```bash
#!/bin/bash
# ~/float-hub-operations/floatctl-rs/evna/run-remote-mcp.sh

set -euo pipefail

# Load environment variables
cd ~/float-hub-operations/floatctl-rs/evna
source .env

# Run evna through Supergateway (stdio → SSE)
# Supergateway listens on port 3100 by default
exec supergateway \
  --port 3100 \
  --stdio-command "bun" \
  --stdio-args "run,--silent,src/mcp-server.ts"
```

Make it executable:
```bash
chmod +x ~/float-hub-operations/floatctl-rs/evna/run-remote-mcp.sh
```

## Step 3: Configure ngrok Tunnel

Create ngrok configuration for persistent tunnel:

```yaml
# ~/.ngrok2/ngrok.yml (add to existing config)

tunnels:
  evna-mcp:
    proto: http
    addr: 3100
    # Optional: restrict to specific domains (ngrok paid feature)
    # domain: evna.your-domain.com
    # Optional: add basic auth (ngrok paid feature)
    # auth: "username:password"
```

Start the tunnel:
```bash
ngrok start evna-mcp
```

Or run inline (if not using config):
```bash
ngrok http 3100 --authtoken YOUR_NGROK_TOKEN
```

ngrok will output:
```
Forwarding https://abc123.ngrok-free.app -> http://localhost:3100
```

## Step 4: Create systemd/launchd Service (Optional)

For auto-start on Mac Mini boot, create a launchd plist:

```xml
<!-- ~/Library/LaunchAgents/com.float-hub.evna-remote-mcp.plist -->
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>Label</key>
    <string>com.float-hub.evna-remote-mcp</string>

    <key>ProgramArguments</key>
    <array>
        <string>/Users/evan/float-hub-operations/floatctl-rs/evna/run-remote-mcp.sh</string>
    </array>

    <key>WorkingDirectory</key>
    <string>/Users/evan/float-hub-operations/floatctl-rs/evna</string>

    <key>EnvironmentVariables</key>
    <dict>
        <key>PATH</key>
        <string>/Users/evan/.bun/bin:/Users/evan/.cargo/bin:/opt/homebrew/bin:/usr/local/bin:/usr/bin:/bin</string>
    </dict>

    <key>RunAtLoad</key>
    <true/>

    <key>KeepAlive</key>
    <true/>

    <key>StandardOutPath</key>
    <string>/Users/evan/.evna-remote-mcp.log</string>

    <key>StandardErrorPath</key>
    <string>/Users/evan/.evna-remote-mcp.error.log</string>
</dict>
</plist>
```

Load the service:
```bash
launchctl load ~/Library/LaunchAgents/com.float-hub.evna-remote-mcp.plist
launchctl start com.float-hub.evna-remote-mcp
```

## Step 5: Configure Clients

### Claude Desktop (Laptop)

Update `~/Library/Application Support/Claude/claude_desktop_config.json`:

```json
{
  "mcpServers": {
    "evna-remote": {
      "url": "https://abc123.ngrok-free.app/sse",
      "transport": "sse"
    }
  }
}
```

### Claude Mobile/Web

When Claude mobile/web supports remote MCP (upcoming feature), use the same ngrok URL.

## Step 6: Test the Setup

### Option A: One Command (Recommended)

```bash
# From anywhere (if floatctl is installed globally)
floatctl evna remote

# Or with custom options
floatctl evna remote --port 3100 --ngrok-domain evna.your-domain.com
```

This single command:
- Checks all dependencies (Supergateway, bun, ngrok)
- Starts Supergateway with evna
- Starts ngrok tunnel
- Shows you the public URL
- Handles Ctrl+C cleanup

### Option B: Manual Steps

1. **Start the server** (Mac Mini):
   ```bash
   ~/float-hub-operations/floatctl-rs/evna/run-remote-mcp.sh
   ```

2. **Start ngrok tunnel**:
   ```bash
   ngrok start evna-mcp
   # Or: ngrok http 3100
   ```

3. **Test with curl**:
   ```bash
   curl https://abc123.ngrok-free.app/sse
   # Should return SSE connection
   ```

4. **Test from Claude Desktop**:
   - Restart Claude Desktop
   - Type: "Use evna to search for 'karen hat check protocol'"
   - Should see results from your Mac Mini's database

## Troubleshooting

### Check Supergateway logs
```bash
tail -f ~/.evna-remote-mcp.log
```

### Check ngrok tunnel status
```bash
# Visit ngrok web UI
open http://localhost:4040
```

### Test stdio server directly
```bash
cd ~/float-hub-operations/floatctl-rs/evna
bun run src/mcp-server.ts
# Should wait for stdin input
```

### Test SSE endpoint
```bash
curl -N https://abc123.ngrok-free.app/sse \
  -H "Accept: text/event-stream"
```

## Security Considerations

1. **ngrok auth**: Use ngrok's basic auth or OAuth (paid feature)
   ```yaml
   tunnels:
     evna-mcp:
       auth: "username:strong-password"
   ```

2. **Database access**: Ensure your Mac Mini's Supabase credentials are secure

3. **Firewall**: Mac Mini should only expose ngrok tunnel, not direct port 3100

4. **Rate limiting**: Consider adding rate limits to prevent abuse

## Cost Optimization

With ngrok paid account:
- Use reserved domains (e.g., `evna.your-domain.com`) for stable URLs
- Enable IP whitelisting to restrict access to your devices
- Set up OAuth for better authentication

## Alternative: Tailscale + Local MCP

If you prefer private network instead of public tunnel:

1. Install Tailscale on Mac Mini + laptop
2. Use Tailscale IP directly in MCP config
3. No ngrok needed, but mobile/web won't work

```json
{
  "mcpServers": {
    "evna-remote": {
      "url": "http://100.x.x.x:3100/sse",
      "transport": "sse"
    }
  }
}
```

## Maintenance

### Update evna
```bash
cd ~/float-hub-operations/floatctl-rs/evna
git pull
bun install
launchctl restart com.float-hub.evna-remote-mcp
```

### Check service status
```bash
launchctl list | grep evna
```

### View logs
```bash
tail -f ~/.evna-remote-mcp.log
tail -f ~/.evna-remote-mcp.error.log
```

## Next Steps

1. Test on laptop first (local ngrok)
2. Move to Mac Mini once stable
3. Set up launchd for auto-start
4. Document your ngrok URL in password manager
5. Test from mobile when Claude adds remote MCP support

---

**Philosophy**: "Run consistently" beats "run locally" - Mac Mini becomes your personal evna-as-a-service, always available, always synced.
