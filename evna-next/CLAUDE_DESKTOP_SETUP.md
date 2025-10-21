# EVNA-Next MCP Server for Claude Desktop

## ✅ Setup Complete!

EVNA-Next is now configured as an MCP server that Claude Desktop can use.

## 🔧 Configuration

The MCP server has been added to your Claude Desktop config at:
```
~/Library/Application Support/Claude/claude_desktop_config.json
```

Configuration:
```json
{
  "mcpServers": {
    "evna-next": {
      "command": "npm",
      "args": [
        "run",
        "--silent",
        "--prefix",
        "/Users/evan/float-hub-operations/floatctl-rs/evna-next",
        "mcp-server"
      ]
    }
  }
}
```

## 🚀 Available Tools in Claude Desktop

Once you restart Claude Desktop, you'll have access to these EVNA tools:

### 1. **brain_boot**
Morning brain boot with comprehensive context synthesis:
- **Semantic search** of conversation history (pgvector)
- **GitHub PR/issue status** (your open PRs and assigned issues)
- **Daily notes** from ~/.evans-notes/daily
- **Recent activity** summary

**Example usage in Claude Desktop:**
```
Good morning! Can you do a brain boot for rangle/pharmacy?
My GitHub username is e-schultz.
```

### 2. **semantic_search**
Direct semantic search across your conversation history:
- Search by natural language queries
- Filter by project name
- Filter by date range
- Adjustable similarity threshold

**Example usage:**
```
Search for recent conversations about GP notifications and assessment flows
```

## 📝 How to Activate

1. **Quit Claude Desktop** completely (Cmd+Q)
2. **Restart Claude Desktop**
3. Look for the 🔌 icon in Claude Desktop - you should see "evna-next" connected
4. Start using the tools in your conversations!

## 🐛 Troubleshooting

If the server doesn't connect:

1. **Check the logs**: Claude Desktop → Settings → Developer → View Logs
2. **Test the server manually**:
   ```bash
   cd /Users/evan/float-hub-operations/floatctl-rs/evna-next
   npm run mcp-server
   ```
   - You should see: "🧠 EVNA-Next MCP Server ready"
   - Press Ctrl+C to stop

3. **Verify environment variables** in `.env`:
   - SUPABASE_URL
   - SUPABASE_SERVICE_KEY
   - OPENAI_API_KEY

4. **Check the config file**:
   ```bash
   cat ~/Library/Application\ Support/Claude/claude_desktop_config.json
   ```

## 💡 Tips

- **brain_boot** works best when you provide context:
  - Your GitHub username (for PR/issue status)
  - Project name (for filtering conversations)
  - What you're looking for (e.g., "where did I leave off")

- **Daily notes** are automatically included if they exist in `~/.evans-notes/daily/`
  - Format: `YYYY-MM-DD.md`
  - Extracts standup updates, PR status, and focus areas

- **GitHub integration** shows:
  - Open PRs with review status
  - CI check status (passing/failing/pending)
  - Assigned issues

## 🎯 Example Brain Boot Prompt

```
Good morning! It's Monday, October 21st.

Can you do a brain boot for the rangle/pharmacy project?
My GitHub username is e-schultz.

I want to see:
- My open PRs and their status
- Issues assigned to me
- What I was working on last week
- Context from my daily notes
```

The agent will synthesize all this information into a comprehensive morning briefing!
