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

// MCP server
export { evnaNextMcpServer, createEvnaMcpServer } from "./interfaces/mcp.js";

// CLI interface
export { main } from "./interfaces/cli.js";
