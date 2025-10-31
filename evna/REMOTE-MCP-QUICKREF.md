# EVNA Remote MCP - Quick Reference

## One-Command Setup

1. **Configure .env** (one time):
   ```bash
   cd ~/float-hub-operations/floatctl-rs/evna
   cp .env.example .env
   # Edit .env and set:
   #   EVNA_NGROK_DOMAIN=evna.ngrok.app
   #   EVNA_NGROK_AUTH=username:password
   ```

2. **Start server**:
   ```bash
   floatctl evna remote
   ```

That's it! This command:
1. ✅ Loads config from evna/.env
2. ✅ Checks dependencies (Supergateway, bun, ngrok)
3. 🚀 Starts Supergateway (stdio → SSE)
4. 🌐 Starts ngrok tunnel with your domain + auth
5. 📋 Shows exact config for Claude Desktop
6. 🛑 Handles Ctrl+C cleanup

## Configuration Priority

Settings are loaded in this order (later overrides earlier):
1. **~/.ngrok2/ngrok.yml** - ngrok's default config
2. **evna/.env** - EVNA-specific settings (recommended)
3. **CLI arguments** - One-off overrides

## Environment Variables (.env)

```bash
# ngrok reserved domain (paid feature)
EVNA_NGROK_DOMAIN=evna.ngrok.app

# ngrok basic auth (format: username:password)
EVNA_NGROK_AUTH=your_username:your_strong_password

# ngrok authtoken (optional - usually in ~/.ngrok2/ngrok.yml)
EVNA_NGROK_AUTHTOKEN=your_ngrok_authtoken
```

**Why EVNA_ prefix?** Avoids conflicts if you use ngrok for other projects.

## Command Options

```bash
# Use custom port
floatctl evna remote --port 3200

# Override domain from CLI (bypasses .env)
floatctl evna remote --ngrok-domain another-domain.ngrok.app

# Specify evna path
floatctl evna remote --path ~/custom/evna-location

# Skip ngrok (local network only)
floatctl evna remote --no-tunnel
```

## Get Your Public URL

If you set `EVNA_NGROK_DOMAIN=evna.ngrok.app` in .env, the command will show:
```
📋 Copied to clipboard: https://username:password@evna.ngrok.app/sse
🎯 Public URL: https://evna.ngrok.app/sse

   (URL with auth credentials copied to clipboard)
```

**Clipboard magic**: The authenticated URL (`https://user:pass@domain/sse`) is automatically copied to your clipboard when both `EVNA_NGROK_DOMAIN` and `EVNA_NGROK_AUTH` are set. Perfect for pasting into Claude mobile/web MCP setup.

Otherwise, open http://localhost:4040 in your browser - ngrok web UI shows your public HTTPS URL.

Or use the ngrok API:
```bash
curl -s http://localhost:4040/api/tunnels | jq -r '.tunnels[0].public_url'
```

## Configure Claude Desktop

1. The `floatctl evna remote` command shows the exact config to use
2. Edit `~/Library/Application Support/Claude/claude_desktop_config.json`:

```json
{
  "mcpServers": {
    "evna-remote": {
      "url": "https://evna.ngrok.app/sse",
      "transport": "sse"
    }
  }
}
```

3. Restart Claude Desktop
4. Test: Ask Claude to search your conversation history

**Note:** If you set `EVNA_NGROK_AUTH` in .env, you'll need to include credentials in the URL:
```json
{
  "mcpServers": {
    "evna-remote": {
      "url": "https://username:password@evna.ngrok.app/sse",
      "transport": "sse"
    }
  }
}
```

## Configure Claude Code

Claude Code requires basic auth in headers, not URL. Use the CLI command or config:

**Option 1: CLI (copy/paste from `floatctl evna remote` output)**:
```bash
claude mcp add evna-remote https://evna.ngrok.app/sse --transport sse --header "Authorization: Basic <base64>"
```

**Option 2: Manual config** in `.mcp.json`:
```json
{
  "mcpServers": {
    "evna-remote": {
      "url": "https://evna.ngrok.app/sse",
      "transport": "sse",
      "headers": {
        "Authorization": "Basic <base64_encoded_username:password>"
      }
    }
  }
}
```

The `floatctl evna remote` command outputs the complete CLI command with base64-encoded credentials.

## Configure Claude Mobile/Web

When adding evna as a remote MCP server in Claude mobile or web:

1. Run `floatctl evna remote` (authenticated URL auto-copied to clipboard)
2. Paste the URL from clipboard into the MCP server field
3. The format is: `https://username:password@evna.ngrok.app/sse`

No manual editing needed - the command handles basic auth URL encoding for you!

## Auto-Start on Mac Mini

Create launchd service:

```bash
# Create service file
cat > ~/Library/LaunchAgents/com.float-hub.evna-remote.plist << 'EOF'
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>Label</key>
    <string>com.float-hub.evna-remote</string>

    <key>ProgramArguments</key>
    <array>
        <string>/Users/evan/.cargo/bin/floatctl</string>
        <string>evna</string>
        <string>remote</string>
    </array>

    <key>WorkingDirectory</key>
    <string>/Users/evan</string>

    <key>RunAtLoad</key>
    <true/>

    <key>KeepAlive</key>
    <true/>

    <key>StandardOutPath</key>
    <string>/Users/evan/.evna-remote.log</string>

    <key>StandardErrorPath</key>
    <string>/Users/evan/.evna-remote.error.log</string>
</dict>
</plist>
EOF

# Load and start
launchctl load ~/Library/LaunchAgents/com.float-hub.evna-remote.plist
launchctl start com.float-hub.evna-remote
```

Check status:
```bash
launchctl list | grep evna
tail -f ~/.evna-remote.log
```

## Troubleshooting

### "Supergateway not found"
```bash
npm install -g supergateway
```

### "ngrok not found"
```bash
brew install ngrok
# Or download from https://ngrok.com/download
```

### "bun not found"
```bash
curl -fsSL https://bun.sh/install | bash
```

### Check if server is running
```bash
# Test local SSE endpoint
curl -N http://localhost:3100/sse -H "Accept: text/event-stream"

# Test ngrok tunnel
curl https://your-ngrok-url.ngrok-free.app/sse
```

### View logs
```bash
# If using launchd
tail -f ~/.evna-remote.log
tail -f ~/.evna-remote.error.log

# ngrok web UI
open http://localhost:4040
```

## Reserved Domains (ngrok Paid)

If you have a reserved ngrok domain:

```bash
# Use your custom domain
floatctl evna remote --ngrok-domain evna.your-domain.com
```

Benefits:
- Stable URL (doesn't change on restart)
- Cleaner URL for sharing
- Can add to password manager once

## Security

### Option 1: ngrok Basic Auth (Paid)

Add to `~/.ngrok2/ngrok.yml`:
```yaml
tunnels:
  evna:
    proto: http
    addr: 3100
    auth: "username:strong-password"
```

Then start with config:
```bash
ngrok start evna
```

### Option 2: IP Whitelist (Paid)

Add to `~/.ngrok2/ngrok.yml`:
```yaml
tunnels:
  evna:
    proto: http
    addr: 3100
    ip_restriction:
      allow_cidrs:
        - YOUR_IP/32
```

### Option 3: Tailscale (Private Network)

Skip ngrok entirely, use Tailscale IPs:

```bash
# Start without ngrok
floatctl evna remote --no-tunnel

# Connect from other devices via Tailscale IP
# Config: http://100.x.x.x:3100/sse
```

## Useful Commands

```bash
# Check evna status
floatctl evna status

# Stop server (if running in background)
pkill -f "floatctl evna remote"

# Stop launchd service
launchctl stop com.float-hub.evna-remote

# View ngrok tunnels
curl -s http://localhost:4040/api/tunnels | jq

# Test evna locally (before remote)
cd ~/float-hub-operations/floatctl-rs/evna
bun run mcp-server
```

## Philosophy

**"One command, always online"** - Mac Mini runs evna 24/7, accessible from laptop, mobile, web. No "did I start the server?" anxiety.
