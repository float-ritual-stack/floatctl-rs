/**
 * EVNA-Next MCP Server — stdio entry
 * Used by Claude Desktop and `floatctl evna remote` (via supergateway, during
 * the transition to the native HTTP entry in src/mcp-http.ts).
 *
 * All server construction lives in src/mcp/server-factory.ts — this file only
 * owns the stdio transport hookup and process-level side effects.
 */

// Load .env with fallback chain: ./.env → ~/.floatctl/.env → existing env vars
import { loadEnvWithFallback } from "./lib/env-loader.js";
loadEnvWithFallback();
import { StdioServerTransport } from "@modelcontextprotocol/sdk/server/stdio.js";
import { createEvnaServer } from "./mcp/server-factory.js";
import { startBridgeSyncTrigger } from "./lib/bridge-sync-trigger.js";

async function main() {
  const server = createEvnaServer();
  const transport = new StdioServerTransport();
  await server.connect(transport);
  console.error("🧠 EVNA-Next MCP Server ready (tools + resources)");

  // Start bridge sync trigger (watches for file changes, triggers R2 sync)
  // Debounces writes (5s) to batch rapid changes, then syncs to make AutoRAG current
  startBridgeSyncTrigger({
    enabled: process.env.EVNA_AUTO_SYNC !== "false",  // Opt-out via env
    debounce_ms: 5000,  // 5 second debounce (batch rapid writes)
  });
}

main().catch((error) => {
  console.error("Fatal error:", error);
  process.exit(1);
});
