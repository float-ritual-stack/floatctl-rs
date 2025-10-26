/**
 * EVNA-Next: Public API exports
 * Clean export-only interface - no business logic here
 */

// Core configuration
export { evnaSystemPrompt, createQueryOptions, DEFAULT_MODEL } from "./core/config.js";

// Tool definitions and clients
export {
  brainBootTool,
  semanticSearchTool,
  activeContextTool,
  testTool,
  db,
  embeddings,
} from "./tools/index.js";

// MCP servers
export { evnaNextMcpServer, createEvnaMcpServer } from "./interfaces/mcp.js"; // Internal MCP for Agent SDK
// Note: External MCP for Claude Desktop is in src/mcp-server.ts (standalone stdio server)

// CLI interface
export { main } from "./interfaces/cli.js";
