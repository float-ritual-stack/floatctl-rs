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
import { toolSchemas } from "./registry-zod.js";

// Initialize clients (singleton pattern)
const supabaseUrl = process.env.SUPABASE_URL!;
const supabaseKey = process.env.SUPABASE_SERVICE_KEY!;
const openaiKey = process.env.OPENAI_API_KEY!;

export const db = new DatabaseClient(supabaseUrl, supabaseKey);
export const embeddings = new EmbeddingsClient(openaiKey);

const githubRepo = process.env.GITHUB_REPO || "pharmonline/pharmacy-online";
const brainBoot = new BrainBootTool(db, embeddings, githubRepo);
const search = new PgVectorSearchTool(db, embeddings);
const activeContext = new ActiveContextTool(db);

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
