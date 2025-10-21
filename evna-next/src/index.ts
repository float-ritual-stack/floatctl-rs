/**
 * EVNA-Next: Agent SDK with pgvector RAG
 * Rich context synthesis for the Queer Techno Bard cognitive ecosystem
 */

import "dotenv/config";
import {
  query,
  tool,
  createSdkMcpServer,
  type SDKUserMessage,
} from "@anthropic-ai/claude-agent-sdk";
import { z } from "zod";
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

// Define Brain Boot tool for Agent SDK
const brainBootTool = tool(
  "brain_boot",
  'Morning brain boot: Semantic search + recent context + GitHub PR/issue status synthesis. Use this for "good morning" check-ins or when the user wants to restore context about where they left off on a project.',
  {
    query: z
      .string()
      .describe(
        'Natural language description of what to retrieve context about (e.g., "tuesday morning pharmacy project where did I leave off")',
      ),
    project: z
      .string()
      .optional()
      .describe('Filter by project name (e.g., "rangle/pharmacy")'),
    lookbackDays: z
      .number()
      .optional()
      .describe("How many days to look back (default: 7)"),
    maxResults: z
      .number()
      .optional()
      .describe("Maximum results to return (default: 10)"),
    githubUsername: z
      .string()
      .optional()
      .describe("GitHub username to fetch PR and issue status (e.g., 'evanebb')"),
  },
  async (args: any) => {
    console.log("[brain_boot] Called with args:", args);
    try {
      const result = await brainBoot.boot({
        query: args.query,
        project: args.project,
        lookbackDays: args.lookbackDays ?? 7,
        maxResults: args.maxResults ?? 10,
        githubUsername: args.githubUsername,
      });
      return {
        content: [
          {
            type: "text" as const,
            text: result.summary,
          },
        ],
      };
    } catch (error) {
      console.error("[brain_boot] Error:", error);
      return {
        content: [
          {
            type: "text" as const,
            text: `Error during brain boot: ${error instanceof Error ? error.message : String(error)}`,
          },
        ],
      };
    }
  },
);

// Define semantic search tool
const semanticSearchTool = tool(
  "semantic_search",
  "Semantic search across conversation history using pgvector embeddings. Returns messages that are semantically similar to the query.",
  {
    query: z
      .string()
      .describe(
        "Search query (can be natural language, a question, or keywords)",
      ),
    limit: z
      .number()
      .optional()
      .describe("Maximum number of results (default: 10)"),
    project: z.string().optional().describe("Filter by project name"),
    since: z
      .string()
      .optional()
      .describe(
        'Filter by timestamp (ISO 8601 format, e.g., "2025-10-01T00:00:00Z")',
      ),
    threshold: z
      .number()
      .optional()
      .describe(
        "Similarity threshold 0-1 (default: 0.5, lower = more results)",
      ),
  },
  async (args: any) => {
    console.log("[semantic_search] Called with args:", args);
    try {
      const results = await search.search({
        query: args.query,
        limit: args.limit ?? 10,
        project: args.project,
        since: args.since,
        threshold: args.threshold ?? 0.5,
      });
      const formatted = search.formatResults(results);
      return {
        content: [
          {
            type: "text" as const,
            text: formatted,
          },
        ],
      };
    } catch (error) {
      console.error("[semantic_search] Error:", error);
      return {
        content: [
          {
            type: "text" as const,
            text: `Error during semantic search: ${error instanceof Error ? error.message : String(error)}`,
          },
        ],
      };
    }
  },
);

// Test tool - simple echo
const testTool = tool(
  "test_echo",
  "Simple test tool that echoes back your input",
  {
    message: z.string().describe("Message to echo back"),
  },
  async (args) => {
    console.log("[test_echo] Called with:", args);
    return {
      content: [
        {
          type: "text" as const,
          text: `Echo: ${args.message}`,
        },
      ],
    };
  },
);

// Create MCP server with our tools
const evnaNextMcpServer = createSdkMcpServer({
  name: "evna-next",
  version: "1.0.0",
  tools: [testTool, brainBootTool, semanticSearchTool],
});

// Main agent runner
async function main() {
  console.log("🧠 EVNA-Next: Agent SDK with pgvector RAG");
  console.log("============================================\n");

  // Brain boot with GitHub integration - MCP tools require streaming input!
  async function* generateMessages(): AsyncGenerator<SDKUserMessage> {
    yield {
      type: "user" as const,
      session_id: "", // Will be filled in by SDK
      message: {
        role: "user" as const,
        content:
          "Good morning! Can you give me a brain boot for the rangle/pharmacy project? My GitHub username is e-schultz. Show me my PR status, assigned issues, and what I've been working on.",
      },
      parent_tool_use_id: null,
    };
  }

  console.log("Running brain boot with GitHub integration...\n");

  try {
    const result = await query({
      prompt: generateMessages(), // Use async generator for MCP tools!
      options: {
        mcpServers: {
          "evna-next": evnaNextMcpServer,
        },
        model: "claude-sonnet-4-20250514",
        permissionMode: "bypassPermissions", // Auto-approve all tools
      },
    });

    for await (const message of result) {
      console.log(message);
    }
  } catch (error) {
    console.error("Error running agent:", error);
  }
}

// Run if executed directly
if (import.meta.url === `file://${process.argv[1]}`) {
  main().catch(console.error);
}

export { brainBootTool, semanticSearchTool, evnaNextMcpServer, db, embeddings };
