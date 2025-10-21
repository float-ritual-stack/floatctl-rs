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

// Initialize clients
const supabaseUrl = process.env.SUPABASE_URL!;
const supabaseKey = process.env.SUPABASE_SERVICE_KEY!;
const openaiKey = process.env.OPENAI_API_KEY!;

const db = new DatabaseClient(supabaseUrl, supabaseKey);
const embeddings = new EmbeddingsClient(openaiKey);
const githubRepo = process.env.GITHUB_REPO || "pharmonline/pharmacy-online";
const brainBoot = new BrainBootTool(db, embeddings, githubRepo);
const search = new PgVectorSearchTool(db, embeddings);

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

// Register tools handler
server.setRequestHandler(ListToolsRequestSchema, async () => {
  return {
    tools: [
      {
        name: "brain_boot",
        description: 'Morning brain boot: Semantic search + recent context + GitHub PR/issue status + daily notes synthesis. Use this for "good morning" check-ins or when the user wants to restore context about where they left off on a project.',
        inputSchema: {
          type: "object",
          properties: {
            query: {
              type: "string",
              description: 'Natural language description of what to retrieve context about (e.g., "tuesday morning pharmacy project where did I leave off")',
            },
            project: {
              type: "string",
              description: 'Filter by project name (e.g., "rangle/pharmacy")',
            },
            lookbackDays: {
              type: "number",
              description: "How many days to look back (default: 7)",
            },
            maxResults: {
              type: "number",
              description: "Maximum results to return (default: 10)",
            },
            githubUsername: {
              type: "string",
              description: "GitHub username to fetch PR and issue status (e.g., 'e-schultz')",
            },
          },
          required: ["query"],
        },
      },
      {
        name: "semantic_search",
        description: "Semantic search across conversation history using pgvector embeddings. Returns messages that are semantically similar to the query.",
        inputSchema: {
          type: "object",
          properties: {
            query: {
              type: "string",
              description: "Search query (can be natural language, a question, or keywords)",
            },
            limit: {
              type: "number",
              description: "Maximum number of results (default: 10)",
            },
            project: {
              type: "string",
              description: "Filter by project name",
            },
            since: {
              type: "string",
              description: 'Filter by timestamp (ISO 8601 format, e.g., "2025-10-01T00:00:00Z")',
            },
            threshold: {
              type: "number",
              description: "Similarity threshold 0-1 (default: 0.5, lower = more results)",
            },
          },
          required: ["query"],
        },
      },
    ],
  };
});

// Register call tool handler
server.setRequestHandler(CallToolRequestSchema, async (request) => {
  const { name, arguments: args } = request.params;

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
