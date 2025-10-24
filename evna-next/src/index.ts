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
export { externalMcpServer, createExternalMcpServer } from "./interfaces/mcp-external.js"; // External MCP for Claude Desktop

// CLI interface
export { main } from "./interfaces/cli.js";
