/**
 * External MCP Server for Claude Desktop and other MCP clients
 * Exposes resources (daily notes, etc.) that external clients can access
 *
 * This is separate from the Agent SDK's internal MCP (which exposes tools to the agent).
 * External clients don't have filesystem access, so they need resources.
 */

import { McpServer } from "@modelcontextprotocol/sdk/server/mcp.js";
import { readFile } from "fs/promises";
import { join } from "path";
import { homedir } from "os";

/**
 * Create external MCP server with resources for Claude Desktop
 */
export function createExternalMcpServer() {
  const server = new McpServer({
    name: "evna-next",
    version: "1.0.0",
  });

  // Resource: Today's daily note
  server.registerResource(
    "daily-note-today",
    "evna://daily-note/today",
    {
      title: "Today's Daily Note",
      description: "Returns today's daily note (YYYY-MM-DD.md) from ~/.evans-notes/daily",
      mimeType: "text/markdown",
    },
    async (uri) => {
      try {
        const today = new Date().toISOString().split('T')[0]; // YYYY-MM-DD
        const notePath = join(homedir(), '.evans-notes', 'daily', `${today}.md`);
        const content = await readFile(notePath, 'utf-8');

        return {
          contents: [
            {
              uri: uri.href,
              text: content,
              mimeType: "text/markdown",
            },
          ],
        };
      } catch (error) {
        throw new Error(`Failed to read today's daily note: ${error instanceof Error ? error.message : String(error)}`);
      }
    }
  );

  // TODO: Add more resources here as needed
  // Examples from old implementation:
  // - evna://daily-note/{date} - specific date's note
  // - evna://tldr/today - today's TLDR
  // - evna://tldr/{date} - specific date's TLDR
  // - evna://weekly/{week} - weekly note
  // - evna://bridges/recent - recent bridge documents

  return server;
}

export const externalMcpServer = createExternalMcpServer();
