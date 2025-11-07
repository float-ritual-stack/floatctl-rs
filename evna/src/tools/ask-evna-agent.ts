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
import { createClaudeProjectsContextHook } from "../hooks/claude-projects-context.js";

export interface AskEvnaAgentOptions {
  query: string;
  session_id?: string;
  fork_session?: boolean;
  timeout_ms?: number; // Max time before returning "still processing" message
  include_projects_context?: boolean; // Inject recent Claude projects context (default: true)
  all_projects?: boolean; // Include all projects vs just evna (default: false)
}

export class AskEvnaAgent {
  /**
   * Ask evna a natural language question using Agent SDK
   * Supports multi-turn conversations with native SDK session management
   */
  async ask(options: AskEvnaAgentOptions): Promise<{ response: string; session_id: string; timed_out?: boolean }> {
    const { query: userQuery, session_id, fork_session, timeout_ms } = options;

    console.error("[ask_evna_agent] Query:", userQuery);
    if (session_id) {
      console.error(`[ask_evna_agent] Resuming session: ${session_id}${fork_session ? ' (fork)' : ''}`);
    }
    if (timeout_ms) {
      console.error(`[ask_evna_agent] Timeout set: ${timeout_ms}ms`);
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

    // Add Claude projects context hook (gives EVNA peripheral vision)
    const projectsContextHook = createClaudeProjectsContextHook({
      enabled: options.include_projects_context !== false, // Default: true
      allProjects: options.all_projects || false,           // Default: false (just evna)
    });
    baseOptions.hooks = [...(baseOptions.hooks || []), projectsContextHook];

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
      let timedOut = false;
      let lastAgentMessage: string | undefined;

      // Set up timeout if specified (for MCP calls that need to return quickly)
      const timeoutPromise = timeout_ms
        ? new Promise<void>((resolve) => {
            setTimeout(() => {
              timedOut = true;
              resolve();
            }, timeout_ms);
          })
        : null;

      // Race between query completion and timeout
      const processQuery = (async () => {
        for await (const message of result) {
          // Check if we've timed out
          if (timedOut) {
            console.error("[ask_evna_agent] Timeout reached, returning early");
            break;
          }

          // Extract session ID from init message
          if (message.type === 'system' && message.subtype === 'init') {
            actualSessionId = message.session_id;
            console.error(`[ask_evna_agent] Session ID: ${actualSessionId}`);
          }

          // Capture any partial content for timeout visibility
          // Agent SDK doesn't expose streaming text, so we'll capture tool usage or any text we can find
          if ((message as any).result || (message as any).text) {
            lastAgentMessage = (message as any).result || (message as any).text || lastAgentMessage;
          }

          // Collect final result (contains complete text response)
          if (message.type === 'result' && message.subtype === 'success') {
            responses.push(message.result);
          }
        }
      })();

      // Wait for either completion or timeout
      if (timeoutPromise) {
        await Promise.race([processQuery, timeoutPromise]);
      } else {
        await processQuery;
      }

      // If we timed out, return early message with progress visibility
      if (timedOut && actualSessionId) {
        const progressInfo = lastAgentMessage 
          ? `\n\n**Last activity:**\n${lastAgentMessage.substring(0, 500)}${lastAgentMessage.length > 500 ? '...' : ''}\n`
          : '';

        return {
          response: "🕐 **Query is taking longer than expected...**\n\n" +
                   "EVNA is still processing your request in the background.\n\n" +
                   progressInfo +
                   "\n**To retrieve results:**\n" +
                   `- Call \`ask_evna\` again with \`session_id: "${actualSessionId}"\`\n` +
                   "- Or ask a follow-up question using the session ID\n\n" +
                   "_Session state has been saved._",
          session_id: actualSessionId,
          timed_out: true
        };
      }

      const finalResponse = responses.join("\n");

      // Return response with session ID
      return {
        response: finalResponse,
        session_id: actualSessionId || "unknown",
        timed_out: false
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
