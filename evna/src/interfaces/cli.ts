/**
 * CLI Interface for EVNA
 * Runs Agent SDK queries from command line with Skills, hooks, and full ecosystem support
 *
 * Usage:
 *   bun run src/interfaces/cli.ts "your query here"
 *   bun run src/interfaces/cli.ts "process issue #7" --notify-issue float-ritual-stack/float-hub#7
 */

import "dotenv/config";
import { query, type SDKUserMessage } from "@anthropic-ai/claude-agent-sdk";
import { evnaNextMcpServer } from "./mcp.js";
import { createQueryOptions } from "../core/config.js";
import { homedir } from "os";

/**
 * Parse CLI arguments
 */
function parseArgs() {
  const args = process.argv.slice(2);

  if (args.length === 0) {
    console.error("Usage: bun run src/interfaces/cli.ts <query> [--notify-issue repo/name#number]");
    console.error("\nExamples:");
    console.error('  bun run src/interfaces/cli.ts "help me with X"');
    console.error('  bun run src/interfaces/cli.ts "process issue #7" --notify-issue float-ritual-stack/float-hub#7');
    process.exit(1);
  }

  const userQuery = args[0];
  const notifyIssueIdx = args.indexOf('--notify-issue');
  const notifyIssue = notifyIssueIdx !== -1 ? args[notifyIssueIdx + 1] : undefined;

  return { userQuery, notifyIssue };
}

/**
 * Main CLI runner
 * Processes a single query and outputs results to stdout
 */
async function main() {
  const { userQuery, notifyIssue } = parseArgs();

  console.error("🧠 EVNA: Agent SDK with Skills, Hooks, and RAG");
  console.error("==============================================\n");
  console.error(`Query: ${userQuery}`);
  if (notifyIssue) {
    console.error(`Notify Issue: ${notifyIssue}`);
  }
  console.error("");

  // Generate messages from CLI input
  async function* generateMessages(): AsyncGenerator<SDKUserMessage> {
    yield {
      type: "user" as const,
      session_id: "", // Will be filled in by SDK
      message: {
        role: "user" as const,
        content: userQuery,
      },
      parent_tool_use_id: null,
    };
  }

  try {
    // Configure Agent SDK options with Skills and hooks enabled
    const options = createQueryOptions(evnaNextMcpServer);

    // Enable Skills and filesystem settings
    options.settingSources = ["user", "project"];
    options.allowedTools = [
      ...(options.allowedTools || []),
      "Skill",  // Enable Agent Skills
      "TodoWrite",  // Enable todo tracking
      "SlashCommand"  // Enable slash commands
    ];

    // Set working directory to user's home or current dir
    options.cwd = process.cwd();

    const result = await query({
      prompt: generateMessages(),
      options,
    });

    for await (const message of result) {
      console.log(message);
    }

    console.error("\n✅ Query completed");
  } catch (error) {
    console.error("❌ Error running agent:", error);
    process.exit(1);
  }
}

// Run if executed directly
if (import.meta.url === `file://${process.argv[1]}`) {
  main().catch(console.error);
}

export { main };
