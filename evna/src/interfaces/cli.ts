/**
 * CLI Interface for EVNA
 * Runs Agent SDK queries from command line with JSON output
 */

import "dotenv/config";
import { query, type SDKUserMessage } from "@anthropic-ai/claude-agent-sdk";
import { evnaNextMcpServer } from "./mcp.js";
import { createQueryOptions } from "../core/config.js";

/**
 * Main CLI runner
 * Processes a single query and outputs results to stdout
 */
async function main() {
  console.log("🧠 EVNA-Next: Agent SDK with pgvector RAG");
  console.log("============================================\n");

  // Example query with brain boot
  async function* generateMessages(): AsyncGenerator<SDKUserMessage> {
    yield {
      type: "user" as const,
      session_id: "", // Will be filled in by SDK
      message: {
        role: "user" as const,
        content: `## brain burp
ctx::2025-10-21 @ 03:56:51 PM - [mode::brain boot] - [project::Evna]

testing magic
`,
      },
      parent_tool_use_id: null,
    };
  }

  console.log("Running brain boot with GitHub integration...\n");

  try {
    const result = await query({
      prompt: generateMessages(),
      options: createQueryOptions(evnaNextMcpServer),
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

export { main };
