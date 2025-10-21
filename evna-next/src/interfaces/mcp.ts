/**
 * MCP Server Interface for EVNA
 * Exposes tools via Model Context Protocol
 */

import { createSdkMcpServer } from "@anthropic-ai/claude-agent-sdk";
import { brainBootTool, semanticSearchTool, activeContextTool, testTool } from "../tools/index.js";

/**
 * Create EVNA MCP server with all tools
 * Used by CLI, TUI, and external MCP clients
 */
export function createEvnaMcpServer() {
  return createSdkMcpServer({
    name: "evna-next",
    version: "1.0.0",
    tools: [testTool, brainBootTool, semanticSearchTool, activeContextTool],
  });
}

// Export singleton instance for convenience
export const evnaNextMcpServer = createEvnaMcpServer();
