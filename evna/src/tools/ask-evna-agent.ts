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
import { execFile } from "child_process";
import { promisify } from "util";
import { debug } from "../lib/logger.js";

const execFileAsync = promisify(execFile);

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

    debug("ask_evna_agent", `Query: ${userQuery}`, {
      session_id,
      fork_session,
      timeout_ms
    });

    // Log query to master_stream.jsonl via floatctl ctx (fire and forget, non-blocking)
    this.logQueryToMasterStream(userQuery, session_id).catch(() => {
      // Silently ignore failures - don't break ask_evna if ctx logging fails
    });

    // Configure Agent SDK options
    // Use INTERNAL MCP server (without ask_evna) to prevent fractal recursion
    const { evnaInternalMcpServer } = await import("../interfaces/mcp.js");
    const baseOptions = createQueryOptions(evnaInternalMcpServer) as any;

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

    // Inject Claude projects context + master stream activity
    if (options.include_projects_context !== false) {
      const { getAskEvnaContextInjection, getAllProjectsContextInjection } = await import("../lib/claude-projects-context.js");
      const { getMasterStreamContextInjection } = await import("../lib/master-stream-context.js");

      // Default to all_projects (pharmacy, float-hub, etc) not just evna
      const allProjects = options.all_projects !== false; // Default: true

      // Fetch both Claude projects and master stream context in parallel
      const [claudeProjectsContext, masterStreamContext] = await Promise.all([
        allProjects ? getAllProjectsContextInjection() : getAskEvnaContextInjection(),
        getMasterStreamContextInjection()  // Uses default 15 entries (~2100 tokens)
      ]);

      // Combine both contexts
      const contextInjection = [claudeProjectsContext, masterStreamContext]
        .filter(c => c && c.length > 0)
        .join("\n\n");

      debug("ask_evna_agent", `Context injection: ${contextInjection?.length || 0} chars`, {
        allProjects,
        claudeProjectsChars: claudeProjectsContext?.length || 0,
        masterStreamChars: masterStreamContext?.length || 0,
        preview: contextInjection?.substring(0, 200) || 'NONE'
      });
      
      if (contextInjection && baseOptions.systemPrompt && typeof baseOptions.systemPrompt === 'object') {
        // Wrap context injection with attribution guidance
        const wrappedContext = `
<external_context>
<attribution>
This is EXTERNAL context from multiple sources:
1. **Claude Desktop/Code conversations** - Recent work from other sessions
2. **Activity stream (master_stream.jsonl)** - Cross-machine ctx:: captures, open-webui chats, system events

These are things that happened OUTSIDE this conversation - attribute properly!

**Source attribution**:
- Claude projects → "According to recent Claude Code work..."
- master_stream from "Evans-Mac-mini.local" → "According to activity stream from Mac Mini..."
- master_stream from "open-webui" → "According to open-webui conversation..."
- master_stream with machine field → "According to ctx:: capture from [machine]..."

**Project path heuristics for Claude Code sessions**:
- "float-hub-operations" or "float-hub/*" → kitty (float-hub Claude Code instance)
- ".evna" or ".floatctl/evna" → evna development work
- Other project paths → probably cowboy (other Claude Code sessions)

**How to reference this context**:
- "According to the activity stream, you were working on..."
- "I see from recent Claude Code sessions that..."
- "The master stream shows..."
- Do NOT say "I completed X" for work you don't directly remember doing in this session
</attribution>

${contextInjection}
</external_context>
`.trim();

        // Append to existing system prompt append field
        const originalLength = baseOptions.systemPrompt.append?.length || 0;
        baseOptions.systemPrompt.append = (baseOptions.systemPrompt.append || '') + '\n\n' + wrappedContext;

        debug("ask_evna_agent", `Injected ${wrappedContext.length} chars into systemPrompt.append`, {
          originalLength,
          newLength: baseOptions.systemPrompt.append.length
        });

        // Store injection metadata for debugging
        (baseOptions as any)._contextInjectionDebug = {
          injected: true,
          length: wrappedContext.length,
          timestamp: new Date().toISOString(),
        };
      } else {
        const failureReason = !contextInjection
          ? 'no_context (empty or undefined context injection)'
          : `wrong_systemPrompt_type (expected object with .append field, got ${typeof baseOptions.systemPrompt})`;

        debug("ask_evna_agent", `FAILED to inject - ${failureReason}`, {
          contextTruthy: !!contextInjection,
          contextLength: contextInjection?.length || 0,
          systemPromptType: typeof baseOptions.systemPrompt,
          hasAppend: !!(baseOptions.systemPrompt as any)?.append
        });

        (baseOptions as any)._contextInjectionDebug = {
          injected: false,
          reason: !contextInjection ? 'no_context' : 'wrong_systemPrompt_type',
          contextLength: contextInjection?.length || 0,
          systemPromptType: typeof baseOptions.systemPrompt,
        };
      }
    }

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
            debug("ask_evna_agent", "Timeout reached, returning early");
            break;
          }

          // Extract session ID from init message
          if (message.type === 'system' && message.subtype === 'init') {
            actualSessionId = message.session_id;
            debug("ask_evna_agent", `Session ID: ${actualSessionId}`);
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

      // If we timed out BUT got a complete response, don't return timeout message
      // (handles race condition where both finish at same time)
      if (timedOut && actualSessionId && responses.length === 0) {
        // Try to get partial progress from session log using floatctl
        let progressInfo = '';
        try {
          const floatctlBin = process.env.FLOATCTL_BIN ?? 'floatctl';
          const { stdout } = await execFileAsync(floatctlBin, [
            'claude', 'show', actualSessionId,
            '--last', '2',
            '--no-tools',
            '--format', 'text'
          ], {
            timeout: 5000,
            maxBuffer: 1024 * 1024, // 1MB max
            env: { ...process.env, RUST_LOG: 'off' },
          });

          if (stdout && stdout.trim()) {
            // Extract just the message content (skip session header/summary)
            const lines = stdout.split('\n');
            const messageLines = lines.filter(l =>
              !l.includes('Session:') &&
              !l.includes('Project:') &&
              !l.includes('Branch:') &&
              !l.includes('Started:') &&
              !l.includes('Ended:') &&
              !l.includes('Summary') &&
              !l.includes('Tokens:') &&
              !l.includes('╭─') &&
              !l.includes('╰─') &&
              !l.includes('┌─') &&
              !l.includes('└─') &&
              l.trim().length > 0
            );

            const partialWork = messageLines.slice(0, 20).join('\n'); // Limit to 20 lines
            if (partialWork.length > 0) {
              progressInfo = `\n\n**What EVNA has been doing:**\n${partialWork}\n${partialWork.length > 800 ? '\n_(truncated)_' : ''}\n`;
            }
          }
        } catch (error) {
          // Fallback to old behavior if floatctl fails
          debug("ask_evna_agent", "Failed to get partial progress from floatctl", { error });
          if (lastAgentMessage) {
            progressInfo = `\n\n**Last activity:**\n${lastAgentMessage.substring(0, 500)}${lastAgentMessage.length > 500 ? '...' : ''}\n`;
          }
        }

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
      debug("ask_evna_agent", "Error in ask()", { error });
      throw error;
    }
  }

  /**
   * Log query to master_stream.jsonl via floatctl ctx
   * Non-blocking, fire-and-forget (errors are caught by caller)
   */
  private async logQueryToMasterStream(query: string, session_id?: string): Promise<void> {
    const floatctlBin = process.env.FLOATCTL_BIN ?? 'floatctl';

    // Format message with ctx:: annotation
    const sessionInfo = session_id ? ` [session::${session_id.substring(0, 8)}]` : '';
    const message = `ctx:: ask_evna query${sessionInfo} - [mode::query]\n\n${query}`;

    try {
      await execFileAsync(floatctlBin, ['ctx', message], {
        timeout: 2000, // Quick timeout - don't block ask_evna
        maxBuffer: 1024 * 100, // 100KB max
      });
    } catch (error) {
      // Silently fail - don't disrupt ask_evna if ctx logging fails
      debug("ask_evna_agent", "Failed to log query to master_stream", { error });
    }
  }

  /**
   * Format response for MCP tool return
   */
  static formatMcpResponse(result: { response: string; session_id: string }): string {
    return `${result.response}\n\n---\n**Session ID**: ${result.session_id}`;
  }
}
