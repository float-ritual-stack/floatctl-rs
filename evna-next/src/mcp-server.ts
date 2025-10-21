/**
 * EVNA-Next MCP Server
 * Standalone MCP server for Claude Desktop integration
 */

import "dotenv/config";
import { Server } from "@modelcontextprotocol/sdk/server/index.js";
import { StdioServerTransport } from "@modelcontextprotocol/sdk/server/stdio.js";
import {
  CallToolRequestSchema,
  ListToolsRequestSchema,
} from "@modelcontextprotocol/sdk/types.js";
import { DatabaseClient } from "./lib/db.js";
import { EmbeddingsClient } from "./lib/embeddings.js";
import { BrainBootTool } from "./tools/brain-boot.js";
import { PgVectorSearchTool } from "./tools/pgvector-search.js";
import { ActiveContextTool } from "./tools/active-context.js";
import { toMcpTools } from "./tools/registry-zod.js";

// Initialize clients
const supabaseUrl = process.env.SUPABASE_URL!;
const supabaseKey = process.env.SUPABASE_SERVICE_KEY!;
const openaiKey = process.env.OPENAI_API_KEY!;

const db = new DatabaseClient(supabaseUrl, supabaseKey);
const embeddings = new EmbeddingsClient(openaiKey);
const githubRepo = process.env.GITHUB_REPO || "pharmonline/pharmacy-online";
const brainBoot = new BrainBootTool(db, embeddings, githubRepo);
const search = new PgVectorSearchTool(db, embeddings);
const activeContext = new ActiveContextTool(db);

// Create MCP server
const server = new Server(
  {
    name: "evna-next",
    version: "1.0.0",
  },
  {
    capabilities: {
      tools: {},
    },
  }
);

// Register tools handler - auto-wired from Zod schemas (converted to JSON)
server.setRequestHandler(ListToolsRequestSchema, async () => {
  return {
    tools: toMcpTools(),
  };
});

// Register call tool handler
server.setRequestHandler(CallToolRequestSchema, async (request) => {
  const { name, arguments: args = {} } = request.params;

  // Note: No console logging during tool execution - MCP uses stderr for JSON-RPC

  try {
    if (name === "brain_boot") {
      const result = await brainBoot.boot({
        query: args.query as string,
        project: args.project as string | undefined,
        lookbackDays: (args.lookbackDays as number | undefined) ?? 7,
        maxResults: (args.maxResults as number | undefined) ?? 10,
        githubUsername: args.githubUsername as string | undefined,
      });

      return {
        content: [
          {
            type: "text",
            text: result.summary,
          },
        ],
      };
    } else if (name === "semantic_search") {
      const results = await search.search({
        query: args.query as string,
        limit: (args.limit as number | undefined) ?? 10,
        project: args.project as string | undefined,
        since: args.since as string | undefined,
        threshold: (args.threshold as number | undefined) ?? 0.5,
      });

      const formatted = search.formatResults(results);

      return {
        content: [
          {
            type: "text",
            text: formatted,
          },
        ],
      };
    } else if (name === "active_context") {
      const result = await activeContext.query({
        query: args.query as string | undefined,
        capture: args.capture as string | undefined,
        limit: (args.limit as number | undefined) ?? 10,
        project: args.project as string | undefined,
        client_type: args.client_type as 'desktop' | 'claude_code' | undefined,
        include_cross_client: (args.include_cross_client as boolean | undefined) ?? true,
      });

      return {
        content: [
          {
            type: "text",
            text: result,
          },
        ],
      };
    } else {
      throw new Error(`Unknown tool: ${name}`);
    }
  } catch (error) {
    // Note: No console logging here - return error via MCP protocol
    return {
      content: [
        {
          type: "text",
          text: `Error: ${error instanceof Error ? error.message : String(error)}`,
        },
      ],
      isError: true,
    };
  }
});

// Start the server
async function main() {
  const transport = new StdioServerTransport();
  await server.connect(transport);
  console.error("🧠 EVNA-Next MCP Server ready");
}

main().catch((error) => {
  console.error("Fatal error:", error);
  process.exit(1);
});
