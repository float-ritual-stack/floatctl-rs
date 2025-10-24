/**
 * EVNA-Next MCP Server
 * Standalone MCP server for Claude Desktop integration
 * Exposes both tools (brain_boot, semantic_search, active_context) and resources (daily notes, etc.)
 */

import "dotenv/config";
import { Server } from "@modelcontextprotocol/sdk/server/index.js";
import { StdioServerTransport } from "@modelcontextprotocol/sdk/server/stdio.js";
import {
  CallToolRequestSchema,
  ListToolsRequestSchema,
  ListResourcesRequestSchema,
  ReadResourceRequestSchema,
} from "@modelcontextprotocol/sdk/types.js";
import { readFile } from "fs/promises";
import { join } from "path";
import { homedir } from "os";
// Import tool instances and business logic from shared module
import { brainBoot, search, activeContext } from "./tools/index.js";
import { toMcpTools } from "./tools/registry-zod.js";

// Create MCP server
const server = new Server(
  {
    name: "evna-next",
    version: "1.0.0",
  },
  {
    capabilities: {
      tools: {},
      resources: {},
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

// Register resources handler
server.setRequestHandler(ListResourcesRequestSchema, async () => {
  return {
    resources: [
      {
        uri: "daily://today",
        name: "Today's Daily Note",
        description: "Returns today's daily note (YYYY-MM-DD.md) from ~/.evans-notes/daily",
        mimeType: "text/markdown",
      },
      // TODO: Add more resources:
      // - daily://{date} - specific date's note (e.g., daily://2025-10-23)
      // - tldr://today - today's TLDR
      // - tldr://{date} - specific date's TLDR
      // - weekly://{week} - weekly note
      // - bridges://recent - recent bridge documents
    ],
  };
});

// Register read resource handler
server.setRequestHandler(ReadResourceRequestSchema, async (request) => {
  const uri = request.params.uri;

  try {
    if (uri === "daily://today") {
      const today = new Date().toISOString().split('T')[0]; // YYYY-MM-DD
      const notePath = join(homedir(), '.evans-notes', 'daily', `${today}.md`);
      const content = await readFile(notePath, 'utf-8');

      return {
        contents: [
          {
            uri,
            mimeType: "text/markdown",
            text: content,
          },
        ],
      };
    } else {
      throw new Error(`Unknown resource URI: ${uri}`);
    }
  } catch (error) {
    throw new Error(`Failed to read resource ${uri}: ${error instanceof Error ? error.message : String(error)}`);
  }
});

// Start the server
async function main() {
  const transport = new StdioServerTransport();
  await server.connect(transport);
  console.error("🧠 EVNA-Next MCP Server ready (tools + resources)");
}

main().catch((error) => {
  console.error("Fatal error:", error);
  process.exit(1);
});
