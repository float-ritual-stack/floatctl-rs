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
import { readFile, readdir } from "fs/promises";
import { join } from "path";
import { homedir } from "os";
// Import tool instances and business logic from shared module
import { brainBoot, search, activeContext, r2Sync, askEvna } from "./tools/index.js";
import { toMcpTools } from "./tools/registry-zod.js";

// Detect instance type from environment variable
// Maps EVNA_INSTANCE to client_type for active context tagging
const INSTANCE_MAP: Record<string, 'desktop' | 'claude_code'> = {
  daddy: 'desktop',      // Claude Desktop
  kitty: 'claude_code',  // Float Hub / Claude Code
  cowboy: 'claude_code', // Other console sessions
};

const evnaInstance = process.env.EVNA_INSTANCE;
const detectedClientType = evnaInstance ? INSTANCE_MAP[evnaInstance] : undefined;

// Log instance detection on startup (stderr safe for MCP)
if (evnaInstance) {
  console.error(`[evna] Instance detected: ${evnaInstance} → client_type: ${detectedClientType || 'unknown'}`);
}

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
        // Use explicit arg > detected instance > heuristic fallback
        client_type: (args.client_type as 'desktop' | 'claude_code' | undefined) ?? detectedClientType,
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
    } else if (name === "r2_sync") {
      const operation = args.operation as string;
      let result: string;

      switch (operation) {
        case "status":
          result = await r2Sync.status({
            daemon_type: args.daemon_type as 'daily' | 'dispatch' | 'all' | undefined,
          });
          break;
        case "trigger":
          result = await r2Sync.trigger({
            daemon_type: args.daemon_type as 'daily' | 'dispatch' | 'all' | undefined,
            wait: args.wait as boolean | undefined,
          });
          break;
        case "start":
          result = await r2Sync.start({
            daemon_type: args.daemon_type as 'daily' | 'dispatch' | 'all' | undefined,
          });
          break;
        case "stop":
          result = await r2Sync.stop({
            daemon_type: args.daemon_type as 'daily' | 'dispatch' | 'all' | undefined,
          });
          break;
        case "logs":
          result = await r2Sync.logs({
            daemon_type: args.daemon_type as 'daily' | 'dispatch',
            lines: args.lines as number | undefined,
          });
          break;
        default:
          throw new Error(`Unknown operation: ${operation}`);
      }

      return {
        content: [
          {
            type: "text",
            text: result,
          },
        ],
      };
    } else if (name === "ask_evna") {
      const result = await askEvna.ask({
        query: args.query as string,
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
      {
        uri: "daily://recent",
        name: "Recent Daily Notes (Last 3 Days)",
        description: "Returns last 3 days of daily notes concatenated with date headers",
        mimeType: "text/markdown",
      },
      {
        uri: "daily://week",
        name: "This Week's Daily Notes (Last 7 Days)",
        description: "Returns last 7 days of daily notes concatenated with date headers",
        mimeType: "text/markdown",
      },
      {
        uri: "daily://list",
        name: "Available Daily Notes",
        description: "Returns JSON list of available daily notes (last 30 days)",
        mimeType: "application/json",
      },
      // TODO: Future resources:
      // - notes://{path} - template for any note (e.g., notes://bridges/restoration.md)
      // - tldr://recent - TLDR summaries
      // - bridges://recent - recent bridge documents
    ],
  };
});

// Register read resource handler
server.setRequestHandler(ReadResourceRequestSchema, async (request) => {
  const uri = request.params.uri;
  const dailyDir = join(homedir(), '.evans-notes', 'daily');

  try {
    // Static: daily://today
    if (uri === "daily://today") {
      const today = new Date().toISOString().split('T')[0]; // YYYY-MM-DD
      const notePath = join(dailyDir, `${today}.md`);
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
    }

    // Static: daily://recent (last 3 days concatenated)
    if (uri === "daily://recent") {
      const recentDates: string[] = [];
      for (let i = 0; i < 3; i++) {
        const d = new Date();
        d.setDate(d.getDate() - i);
        recentDates.push(d.toISOString().split('T')[0]);
      }

      const sections = await Promise.all(
        recentDates.map(async (date) => {
          const notePath = join(dailyDir, `${date}.md`);
          try {
            const content = await readFile(notePath, 'utf-8');
            return `# ${date}\n\n${content}`;
          } catch (err) {
            return `# ${date}\n\n*(No note found)*`;
          }
        })
      );

      const combined = sections.join('\n\n---\n\n');

      return {
        contents: [
          {
            uri,
            mimeType: "text/markdown",
            text: combined,
          },
        ],
      };
    }

    // Static: daily://week (last 7 days concatenated)
    if (uri === "daily://week") {
      const weekDates: string[] = [];
      for (let i = 0; i < 7; i++) {
        const d = new Date();
        d.setDate(d.getDate() - i);
        weekDates.push(d.toISOString().split('T')[0]);
      }

      const sections = await Promise.all(
        weekDates.map(async (date) => {
          const notePath = join(dailyDir, `${date}.md`);
          try {
            const content = await readFile(notePath, 'utf-8');
            return `# ${date}\n\n${content}`;
          } catch (err) {
            return `# ${date}\n\n*(No note found)*`;
          }
        })
      );

      const combined = sections.join('\n\n---\n\n');

      return {
        contents: [
          {
            uri,
            mimeType: "text/markdown",
            text: combined,
          },
        ],
      };
    }

    // Static: daily://list (JSON of last 30 days)
    if (uri === "daily://list") {
      const files = await readdir(dailyDir);
      const noteFiles = files
        .filter((f) => /^\d{4}-\d{2}-\d{2}\.md$/.test(f))
        .map((f) => f.replace('.md', ''))
        .sort()
        .reverse()
        .slice(0, 30);

      return {
        contents: [
          {
            uri,
            mimeType: "application/json",
            text: JSON.stringify({ notes: noteFiles }, null, 2),
          },
        ],
      };
    }

    throw new Error(`Unknown resource URI: ${uri}`);
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
