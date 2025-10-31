/**
 * MCP Server Interface for EVNA
 * Exposes tools via Model Context Protocol
 */

import { createSdkMcpServer } from "@anthropic-ai/claude-agent-sdk";
import { brainBootTool, semanticSearchTool, activeContextTool, testTool } from "../tools/index.js";
import { readFile } from "fs/promises";
import { join } from "path";
import { homedir } from "os";

/**
 * Create EVNA MCP server with all tools
 * Used by CLI, TUI, and external MCP clients
 */
export function createEvnaMcpServer() {
  return createSdkMcpServer({
    name: "evna-next",
    version: "1.0.0",
    tools: [testTool, brainBootTool, semanticSearchTool, activeContextTool],
    // TODO: MCP resources not yet supported by Agent SDK
    // Commenting out until SDK supports resources property
    // For now, use brain_boot with includeDailyNote=true parameter
    // resources: [
    //   {
    //     uri: "evna://daily-note/today",
    //     name: "Today's daily note",
    //     description: "Returns today's daily note (YYYY-MM-DD.md) from ~/.evans-notes/daily",
    //     mimeType: "text/markdown",
    //     async read() {
    //       try {
    //         const today = new Date().toISOString().split('T')[0]; // YYYY-MM-DD
    //         const notePath = join(homedir(), '.evans-notes', 'daily', `${today}.md`);
    //         const content = await readFile(notePath, 'utf-8');
    //         return { contents: [{ uri: "evna://daily-note/today", mimeType: "text/markdown", text: content }] };
    //       } catch (error) {
    //         throw new Error(`Failed to read today's daily note: ${error instanceof Error ? error.message : String(error)}`);
    //       }
    //     },
    //   },
    // ],
  });
}

// Export singleton instance for convenience
export const evnaNextMcpServer = createEvnaMcpServer();
