/**
 * Ask EVNA Agent (Agent SDK version)
 *
 * Ultra-thin wrapper around Agent SDK query() for ask_evna MCP tool.
 * Agent SDK handles: sessions, tools, context, Skills, hooks, slash commands.
 * We handle: session passthrough, MCP response formatting.
 *
 * Migration gains:
 * - 90%+ token reduction (Agent SDK context isolation)
 * - Skills support (~/.evna/skills/)
 * - Slash commands (~/.evna/commands/)
 * - Proper hooks (via plugin, Phase 2)
 * - TodoWrite, subagents, all Agent SDK features
 */

import { query, type SDKUserMessage } from "@anthropic-ai/claude-agent-sdk";
import { createQueryOptions } from "../core/config.js";
import { homedir } from "os";
import { join } from "path";

export interface AskEvnaAgentOptions {
  query: string;
  session_id?: string;
  fork_session?: boolean;
}

export class AskEvnaAgent {
  /**
   * Ask evna a natural language question using Agent SDK
   * Supports multi-turn conversations with native SDK session management
   */
  async ask(options: AskEvnaAgentOptions): Promise<{ response: string; session_id: string }> {
    const { query: userQuery, session_id, fork_session } = options;

    console.error("[ask_evna_agent] Query:", userQuery);
    if (session_id) {
      console.error(`[ask_evna_agent] Resuming session: ${session_id}${fork_session ? ' (fork)' : ''}`);
    }

    // Configure Agent SDK options
    // Lazy-load MCP server to avoid circular dependency
    const { evnaNextMcpServer } = await import("../interfaces/mcp.js");
    const baseOptions = createQueryOptions(evnaNextMcpServer) as any;

    // Enable Skills, TodoWrite, SlashCommand
    baseOptions.settingSources = ["user", "project"];
    baseOptions.allowedTools = [
      ...(baseOptions.allowedTools || []),
      "Skill",
      "TodoWrite",
      "SlashCommand"
    ];

    // Set working directory to ~/.evna for global skills/commands
    baseOptions.cwd = join(homedir(), '.evna');

    // Add session options if resuming
    if (session_id) {
      baseOptions.resume = session_id;
      if (fork_session) {
        baseOptions.forkSession = true;
      }
    }

    // Generate messages for Agent SDK
    async function* generateMessages(): AsyncGenerator<SDKUserMessage> {
      yield {
        type: "user" as const,
        session_id: "", // SDK will fill this
        message: {
          role: "user" as const,
          content: userQuery,
        },
        parent_tool_use_id: null,
      };
    }

    try {
      // Call Agent SDK query
      const result = query({
        prompt: generateMessages(),
        options: baseOptions,
      });

      // Collect responses and extract session_id
      const responses: string[] = [];
      let actualSessionId: string | undefined;

      for await (const message of result) {
        // Extract session ID from init message
        if (message.type === 'system' && message.subtype === 'init') {
          actualSessionId = message.session_id;
          console.error(`[ask_evna_agent] Session ID: ${actualSessionId}`);
        }

        // Collect final result (contains complete text response)
        if (message.type === 'result' && message.subtype === 'success') {
          responses.push(message.result);
        }
      }

      const finalResponse = responses.join("\n");

      // Return response with session ID
      return {
        response: finalResponse,
        session_id: actualSessionId || "unknown"
      };
    } catch (error) {
      console.error("[ask_evna_agent] Error:", error);
      throw error;
    }
  }

  /**
   * Format response for MCP tool return
   */
  static formatMcpResponse(result: { response: string; session_id: string }): string {
    return `${result.response}\n\n---\n**Session ID**: ${result.session_id}`;
  }
}
