/**
 * Tool definitions for EVNA
 * Agent SDK tool() wrappers around business logic
 */

import { tool } from "@anthropic-ai/claude-agent-sdk";
import { z } from "zod";
import { DatabaseClient } from "../lib/db.js";
import { EmbeddingsClient } from "../lib/embeddings.js";
import { BrainBootTool } from "./brain-boot.js";
import { PgVectorSearchTool } from "./pgvector-search.js";
import { ActiveContextTool } from "./active-context.js";
import { R2SyncTool } from "./r2-sync.js";
import { AskEvnaTool } from "./ask-evna.js";
import { toolSchemas } from "./registry-zod.js";
import workspaceContext from "../config/workspace-context.json";

/**
 * Get required environment variable with validation
 * Throws helpful error if variable is missing
 */
function getRequiredEnv(name: string): string {
  const value = process.env[name];
  if (!value) {
    throw new Error(
      `Missing required environment variable: ${name}\n\n` +
      `Please set this variable in your .env file or environment.\n\n` +
      `Required variables for EVNA:\n` +
      `  - SUPABASE_URL: Your Supabase project URL\n` +
      `  - SUPABASE_SERVICE_KEY: Your Supabase service role key\n` +
      `  - OPENAI_API_KEY: Your OpenAI API key\n\n` +
      `Example .env file:\n` +
      `  SUPABASE_URL=https://your-project.supabase.co\n` +
      `  SUPABASE_SERVICE_KEY=your-service-key\n` +
      `  OPENAI_API_KEY=sk-...`
    );
  }
  return value;
}

// Initialize clients (singleton pattern)
const supabaseUrl = getRequiredEnv("SUPABASE_URL");
const supabaseKey = getRequiredEnv("SUPABASE_SERVICE_KEY");
const openaiKey = getRequiredEnv("OPENAI_API_KEY");

export const db = new DatabaseClient(supabaseUrl, supabaseKey);
export const embeddings = new EmbeddingsClient(openaiKey);

// Use GITHUB_REPO env var, or fall back to workspace-context default
const githubRepo = process.env.GITHUB_REPO ||
  (workspaceContext.projects.pharmacy as any)?.repo;
export const brainBoot = new BrainBootTool(db, embeddings, githubRepo);
export const search = new PgVectorSearchTool(db, embeddings);
export const activeContext = new ActiveContextTool(db);
export const r2Sync = new R2SyncTool();
export const askEvna = new AskEvnaTool(brainBoot, search, activeContext, db);

// Brain Boot tool - semantic search + active context + GitHub integration
export const brainBootTool = tool(
  toolSchemas.brain_boot.name,
  toolSchemas.brain_boot.description,
  toolSchemas.brain_boot.schema.shape,
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

// Semantic search tool - deep pgvector search
export const semanticSearchTool = tool(
  toolSchemas.semantic_search.name,
  toolSchemas.semantic_search.description,
  toolSchemas.semantic_search.schema.shape,
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

// Active context tool - recent activity stream
export const activeContextTool = tool(
  toolSchemas.active_context.name,
  toolSchemas.active_context.description,
  toolSchemas.active_context.schema.shape,
  async (args: any) => {
    console.log("[active_context] Called with args:", args);
    try {
      const result = await activeContext.query({
        query: args.query,
        capture: args.capture,
        limit: args.limit ?? 10,
        project: args.project,
        client_type: args.client_type,
        include_cross_client: args.include_cross_client ?? true,
      });
      return {
        content: [
          {
            type: "text" as const,
            text: result,
          },
        ],
      };
    } catch (error) {
      console.error("[active_context] Error:", error);
      return {
        content: [
          {
            type: "text" as const,
            text: `Error during active context query: ${error instanceof Error ? error.message : String(error)}`,
          },
        ],
      };
    }
  },
);

// R2 Sync tool - daemon management (consolidated)
export const r2SyncTool = tool(
  toolSchemas.r2_sync.name,
  toolSchemas.r2_sync.description,
  toolSchemas.r2_sync.schema.shape,
  async (args: any) => {
    console.log("[r2_sync] Called with args:", args);
    try {
      const operation = args.operation;
      let result: string;

      switch (operation) {
        case "status":
          result = await r2Sync.status({ daemon_type: args.daemon_type });
          break;
        case "trigger":
          result = await r2Sync.trigger({ daemon_type: args.daemon_type, wait: args.wait });
          break;
        case "start":
          result = await r2Sync.start({ daemon_type: args.daemon_type });
          break;
        case "stop":
          result = await r2Sync.stop({ daemon_type: args.daemon_type });
          break;
        case "logs":
          result = await r2Sync.logs({
            daemon_type: args.daemon_type as 'daily' | 'dispatch',
            lines: args.lines
          });
          break;
        default:
          throw new Error(`Unknown operation: ${operation}`);
      }

      return {
        content: [
          {
            type: "text" as const,
            text: result,
          },
        ],
      };
    } catch (error) {
      console.error("[r2_sync] Error:", error);
      return {
        content: [
          {
            type: "text" as const,
            text: `Error performing ${args.operation}: ${error instanceof Error ? error.message : String(error)}`,
          },
        ],
      };
    }
  },
);

// Test tool - simple echo for debugging
export const testTool = tool(
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

// Ask EVNA tool - LLM-driven orchestrator
export const askEvnaTool = tool(
  toolSchemas.ask_evna.name,
  toolSchemas.ask_evna.description,
  toolSchemas.ask_evna.schema.shape,
  async (args: any) => {
    console.log("[ask_evna] Called with args:", args);
    try {
      const result = await askEvna.ask({
        query: args.query,
        session_id: args.session_id,
        fork_session: args.fork_session,
      });

      return {
        content: [
          {
            type: "text" as const,
            text: AskEvnaTool.formatMcpResponse(result),
          },
        ],
      };
    } catch (error) {
      console.error("[ask_evna] Error:", error);
      return {
        content: [
          {
            type: "text" as const,
            text: `Error during ask_evna orchestration: ${error instanceof Error ? error.message : String(error)}`,
          },
        ],
      };
    }
  },
);
