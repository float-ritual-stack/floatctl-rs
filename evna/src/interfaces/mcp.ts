/**
 * MCP Server Interface for EVNA
 * Exposes tools via Model Context Protocol
 */

import { createSdkMcpServer } from "@anthropic-ai/claude-agent-sdk";
import {
  brainBootTool,
  semanticSearchTool,
  activeContextTool,
  askEvnaTool,
  testTool,
  bridgeHealthTool,
  githubReadIssueTool,
  githubCommentIssueTool,
  githubCloseIssueTool,
  githubAddLabelTool,
  githubRemoveLabelTool,
  autoragSearchTool,
  listRecentClaudeSessionsTool,
  readRecentClaudeContextTool,
} from "../tools/index.js";
import { readFile } from "fs/promises";
import { join } from "path";
import { homedir } from "os";

/**
 * Create EVNA MCP server for CLI/TUI (includes ask_evna)
 * Used by CLI and TUI interfaces
 */
export function createEvnaMcpServer() {
  return createSdkMcpServer({
    name: "evna-next",
    version: "1.0.0",
    tools: [
      testTool,
      brainBootTool,
      semanticSearchTool,
      activeContextTool,
      askEvnaTool, // Available to CLI/TUI
      // Internal-only tools
      bridgeHealthTool,
      autoragSearchTool,
      githubReadIssueTool,
      githubCommentIssueTool,
      githubCloseIssueTool,
      githubAddLabelTool,
      githubRemoveLabelTool,
      listRecentClaudeSessionsTool,
      readRecentClaudeContextTool,
    ],
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

/**
 * Create internal MCP server for ask_evna's Agent SDK agent
 * IMPORTANT: Does NOT include ask_evna to prevent fractal recursion!
 *
 * This prevents the "fractal evna" bug where:
 * 1. User calls ask_evna
 * 2. ask_evna spawns Agent SDK agent with MCP tools
 * 3. Agent sees ask_evna tool in MCP server
 * 4. Agent calls ask_evna recursively
 * 5. Infinite loop → hundreds of processes → system overload
 */
export function createInternalMcpServer() {
  return createSdkMcpServer({
    name: "evna-next-internal",
    version: "1.0.0",
    tools: [
      // Basic tools available to ask_evna's agent
      testTool,
      brainBootTool,
      semanticSearchTool,
      activeContextTool,
      // ❌ askEvnaTool INTENTIONALLY EXCLUDED to prevent recursion

      // Internal-only tools
      bridgeHealthTool,
      autoragSearchTool,
      githubReadIssueTool,
      githubCommentIssueTool,
      githubCloseIssueTool,
      githubAddLabelTool,
      githubRemoveLabelTool,
      listRecentClaudeSessionsTool,
      readRecentClaudeContextTool,
    ],
  });
}

// Export singleton instances for convenience
export const evnaNextMcpServer = createEvnaMcpServer();
export const evnaInternalMcpServer = createInternalMcpServer();
