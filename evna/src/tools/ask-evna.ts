/**
 * Ask EVNA Tool
 * LLM-driven orchestration layer that interprets natural language queries
 * and intelligently coordinates existing evna tools
 */

import Anthropic from "@anthropic-ai/sdk";
import { BrainBootTool } from "./brain-boot.js";
import { PgVectorSearchTool } from "./pgvector-search.js";
import { ActiveContextTool } from "./active-context.js";
import { DatabaseClient } from "../lib/db.js";
import { readFile, readdir, mkdir, appendFile } from "fs/promises";
import { join } from "path";
import { homedir } from "os";
import { exec } from "child_process";
import { promisify } from "util";
import { SearchSession } from "../lib/search-session.js";
import { randomUUID } from "crypto";

const execAsync = promisify(exec);

// System prompt for the orchestrator agent
const AGENT_SYSTEM_PROMPT = `You are evna, an agent orchestrator for Evan's work context system.

Available tools (database):
- active_context: Recent activity stream (last few hours to days). Use for "what am I working on now?" or "recent work" queries. Can filter by project.
- semantic_search: Deep historical search (full conversation archive). Use for finding past discussions, patterns across time, or specific topics regardless of when they occurred.
- brain_boot: Multi-source synthesis (semantic + GitHub + daily notes + recent activity). Use for comprehensive context restoration like morning check-ins, returning from breaks, or "where did I leave off?" scenarios.

Available tools (filesystem):
- read_daily_note: Read Evan's daily notes (defaults to today). Use for timelog, daily tasks, reminders, invoice tracking.
- list_recent_claude_sessions: List recent Claude Code sessions with titles. Use for "what conversations did I have?" or "recent Claude sessions".
- search_dispatch: Search float.dispatch content (inbox, imprints). Use for finding specific files, content patterns, or topics in Evan's knowledge base.
  **GREP INFRASTRUCTURE**: Evan built vocabulary and pattern docs:
    • ~/float-hub/float.dispatch/docs/FRONTMATTER-VOCABULARY.md (master registry of types, statuses, context tags, personas)
    • ~/float-hub/float.dispatch/docs/GREP-PATTERNS.md (common grep patterns and when to use grep vs semantic)
  Use these for structural queries ("find all personas", "what types exist?", "list all handbooks").
- read_file: Read any file by path. Use when you need specific file content and have the exact path.

Your job:
1. Understand the query intent (temporal? project-based? semantic? comprehensive? filesystem? structural?)
2. Decide which tool(s) to call (database vs filesystem, one or multiple)
3. Execute tools in appropriate order if chaining is needed
4. Synthesize results into coherent narrative
5. Filter noise (ignore irrelevant tangents)
6. Avoid repeating what user just said

Guidelines:
- For recent/temporal database queries: Use active_context or brain_boot
- For historical/semantic database queries: Use semantic_search
- For daily notes/tasks/reminders: Use read_daily_note
- For recent work sessions: Use list_recent_claude_sessions
- For structural/vocabulary queries: Check FRONTMATTER-VOCABULARY.md first, then use search_dispatch for exact matching
- For grep pattern questions: Reference GREP-PATTERNS.md to learn available patterns
- For finding specific content in float.dispatch: Use search_dispatch
- For reading specific files: Use read_file
- Use search_dispatch for exact frontmatter matching (e.g., "type: handbook", "persona: qtb", "status: active")
- You can chain: search_dispatch to find files → read_file to get full content
- You can mix database + filesystem tools (e.g., semantic_search for topics, then read_file for details)

Respond with synthesis, not raw data dumps. Focus on answering the user's question directly.`;

export interface AskEvnaOptions {
  query: string;
  session_id?: string;      // Resume existing session
  fork_session?: boolean;   // Fork from session_id instead of continuing
}

export class AskEvnaTool {
  private client: Anthropic;
  private transcriptPath: string | null = null;
  private searchSession: SearchSession | null = null;

  constructor(
    private brainBoot: BrainBootTool,
    private search: PgVectorSearchTool,
    private activeContext: ActiveContextTool,
    private db: DatabaseClient
  ) {
    // Initialize Anthropic client
    const apiKey = process.env.ANTHROPIC_API_KEY;
    if (!apiKey) {
      throw new Error(
        "Missing ANTHROPIC_API_KEY environment variable. " +
          "This is required for the ask_evna orchestrator."
      );
    }
    this.client = new Anthropic({ apiKey });
  }

  /**
   * Initialize transcript logging for this ask_evna session
   */
  private async initTranscriptLogging(): Promise<void> {
    if (process.env.EVNA_LOG_TRANSCRIPTS !== 'true') {
      return;
    }

    const logDir = join(homedir(), '.evna', 'logs');
    await mkdir(logDir, { recursive: true });

    const timestamp = new Date().toISOString().replace(/[:.]/g, '-');
    this.transcriptPath = join(logDir, `ask_evna-${timestamp}.jsonl`);

    console.error(`[ask_evna] Transcript logging to: ${this.transcriptPath}`);
  }

  /**
   * Log a message to the transcript file
   */
  private async logTranscript(entry: any): Promise<void> {
    if (!this.transcriptPath) return;

    try {
      await appendFile(this.transcriptPath, JSON.stringify(entry) + '\n');
    } catch (error) {
      console.error('[ask_evna] Failed to write transcript:', error);
    }
  }

  /**
   * Score the quality of search results using LLM semantic understanding
   * This replaces naive heuristics with actual semantic relevance assessment
   */
  private async scoreResultQuality(
    userQuery: string,
    toolName: string,
    resultText: string
  ): Promise<'high' | 'medium' | 'low' | 'none'> {
    // Quick heuristic checks first (avoid LLM call if obviously none)
    if (resultText.includes('**No results found**') ||
        resultText.includes('No matches found') ||
        resultText.trim().length < 50) {
      return 'none';
    }

    // Keyword matching heuristic (avoid LLM call if keywords match)
    // Extract significant words from query (3+ chars, not common words)
    const queryKeywords = userQuery
      .toLowerCase()
      .split(/\s+/)
      .filter(word => word.length >= 3)
      .filter(word => !['the', 'and', 'for', 'what', 'were', 'are', 'from', 'with'].includes(word));

    const resultLower = resultText.toLowerCase();
    const matchedKeywords = queryKeywords.filter(keyword => resultLower.includes(keyword));

    // If most keywords appear, assume at least medium quality
    if (matchedKeywords.length >= Math.ceil(queryKeywords.length * 0.5)) {
      console.error(`[ask_evna] Keyword match: ${matchedKeywords.length}/${queryKeywords.length} keywords found, skipping LLM`);
      return 'medium';
    }

    try {
      // Truncate very long results to stay within token limits
      const truncatedResult = resultText.length > 2000
        ? resultText.substring(0, 2000) + '...[truncated]'
        : resultText;

      const response = await this.client.messages.create({
        model: "claude-sonnet-4-20250514",
        max_tokens: 50,
        temperature: 0,
        messages: [{
          role: "user",
          content: `Rate how relevant these search results are to the user's query.

User query: "${userQuery}"
Tool used: ${toolName}
Results:
${truncatedResult}

Rate the relevance as ONE WORD ONLY:
- "high" if results directly answer the query
- "medium" if results are related but not direct answer
- "low" if results are tangentially related
- "none" if results are unrelated or empty

Rating:`
        }]
      });

      // Extract rating from response
      const rating = response.content
        .find(block => block.type === 'text')
        ?.text.trim().toLowerCase();

      if (rating?.includes('high')) return 'high';
      if (rating?.includes('medium')) return 'medium';
      if (rating?.includes('low')) return 'low';
      if (rating?.includes('none')) return 'none';

      // Fallback: if we can't parse, assume medium
      console.error('[ask_evna] Could not parse quality rating:', rating);
      return 'medium';

    } catch (error) {
      console.error('[ask_evna] Error scoring quality with LLM:', error);
      // Fallback to naive scoring on error
      return resultText.length > 500 ? 'medium' : 'low';
    }
  }

  /**
   * Ask evna a natural language question
   * The orchestrator agent decides which tools to use
   * Supports session management for multi-turn conversations
   */
  async ask(options: AskEvnaOptions): Promise<{ response: string; session_id: string }> {
    const { query, session_id, fork_session } = options;

    // Generate or use session ID
    const actualSessionId = session_id && !fork_session
      ? session_id
      : randomUUID();

    console.error("[ask_evna] Session:", actualSessionId, session_id ? (fork_session ? "(forked)" : "(resumed)") : "(new)");

    // Load existing messages if resuming/forking
    let messages: Anthropic.MessageParam[] = [];
    if (session_id) {
      const session = await this.db.getAskEvnaSession(session_id);
      if (session) {
        messages = session.messages;
        console.error(`[ask_evna] Loaded ${messages.length} messages from session ${session_id}`);
      } else {
        console.error(`[ask_evna] Session ${session_id} not found, starting fresh`);
      }
    }

    // Add new user query
    messages.push({
      role: "user",
      content: query,
    });

    // Initialize transcript logging and search session
    await this.initTranscriptLogging();
    this.searchSession = new SearchSession(query);

    console.error("[ask_evna] Query:", query);
    await this.logTranscript({
      type: "user_query",
      timestamp: new Date().toISOString(),
      query,
      session_id: actualSessionId,
    });

    try {
      // Start agent loop
      let response = await this.client.messages.create({
        model: "claude-sonnet-4-20250514",
        max_tokens: 4096,
        system: AGENT_SYSTEM_PROMPT,
        messages,
        tools: this.defineTools(),
      });

      await this.logTranscript({
        type: "assistant_response",
        timestamp: new Date().toISOString(),
        stop_reason: response.stop_reason,
        content: response.content,
        usage: response.usage,
      });

      // Handle multi-turn tool execution with early termination
      const finalResponse = await this.handleAgentLoop(messages, response);

      await this.logTranscript({
        type: "final_response",
        timestamp: new Date().toISOString(),
        response: finalResponse,
        session_id: actualSessionId,
      });

      // Save session to database
      await this.db.saveAskEvnaSession(actualSessionId, messages);
      console.error(`[ask_evna] Session ${actualSessionId} saved with ${messages.length} messages`);

      return {
        response: finalResponse,
        session_id: actualSessionId
      };
    } catch (error) {
      console.error("[ask_evna] Error:", error);
      await this.logTranscript({
        type: "error",
        timestamp: new Date().toISOString(),
        error: error instanceof Error ? error.message : String(error),
      });
      throw error;
    }
  }

  /**
   * Format response for MCP tool return
   * Extracts duplicate formatting logic from MCP wrappers
   */
  static formatMcpResponse(result: { response: string; session_id: string }): string {
    return `${result.response}\n\n---\n**Session ID**: ${result.session_id}`;
  }

  /**
   * Handle the agent loop - continue calling tools until agent stops
   */
  private async handleAgentLoop(
    messages: Anthropic.MessageParam[],
    response: Anthropic.Message
  ): Promise<string> {
    let currentResponse = response;

    // Loop while agent wants to use tools
    while (currentResponse.stop_reason === "tool_use") {
      console.error("[ask_evna] Agent requesting tool use");

      // Add assistant's response to messages
      messages.push({
        role: "assistant",
        content: currentResponse.content,
      });

      // Execute all requested tools
      const toolResults = await this.executeTools(currentResponse.content);

      // Add tool results to messages
      messages.push({
        role: "user",
        content: toolResults,
      });

      await this.logTranscript({
        type: "tool_results",
        timestamp: new Date().toISOString(),
        results: toolResults,
      });

      // Check early termination heuristics
      if (this.searchSession) {
        const termination = this.searchSession.shouldTerminate();

        if (termination.shouldTerminate) {
          console.error(`[ask_evna] Early termination: ${termination.reason}`);
          await this.logTranscript({
            type: "early_termination",
            timestamp: new Date().toISOString(),
            reason: termination.reason,
            message: termination.message,
            attempts: this.searchSession.getAttempts(),
            totalTokens: this.searchSession.getTotalTokens(),
          });

          // Return graceful negative response
          return this.searchSession.buildNegativeResponse();
        }
      }

      // Continue conversation with tool results
      currentResponse = await this.client.messages.create({
        model: "claude-sonnet-4-20250514",
        max_tokens: 4096,
        system: AGENT_SYSTEM_PROMPT,
        messages,
        tools: this.defineTools(),
      });

      // Update token costs in search session with actual usage
      if (this.searchSession && currentResponse.usage) {
        const attempts = this.searchSession.getAttempts();
        if (attempts.length > 0) {
          const lastAttempt = attempts[attempts.length - 1];
          lastAttempt.tokenCost = currentResponse.usage.input_tokens || 0;
        }
      }

      await this.logTranscript({
        type: "assistant_response",
        timestamp: new Date().toISOString(),
        stop_reason: currentResponse.stop_reason,
        content: currentResponse.content,
        usage: currentResponse.usage,
      });
    }

    // Extract final text response
    return this.extractTextResponse(currentResponse);
  }

  /**
   * Execute tools requested by the agent
   * Calls existing tool instances - no duplication of logic
   */
  private async executeTools(
    content: Anthropic.ContentBlock[]
  ): Promise<Anthropic.ToolResultBlockParam[]> {
    const toolUses = content.filter(
      (block): block is Anthropic.ToolUseBlock => block.type === "tool_use"
    );

    const results = await Promise.all(
      toolUses.map(async (toolUse) => {
        console.error(`[ask_evna] Executing tool: ${toolUse.name}`, toolUse.input);

        await this.logTranscript({
          type: "tool_call",
          timestamp: new Date().toISOString(),
          tool: toolUse.name,
          input: toolUse.input,
        });

        let result: string;

        try {
          switch (toolUse.name) {
            case "active_context": {
              const formatted = await this.activeContext.query(
                toolUse.input as any
              );
              result = formatted;
              break;
            }

            case "semantic_search": {
              const searchResults = await this.search.search(
                toolUse.input as any
              );
              result = this.search.formatResults(searchResults);
              break;
            }

            case "brain_boot": {
              const bootResult = await this.brainBoot.boot(toolUse.input as any);
              result = bootResult.summary;
              break;
            }

            case "read_daily_note": {
              const input = toolUse.input as { date?: string };
              const date = input.date || new Date().toISOString().split("T")[0];
              const notePath = join(
                homedir(),
                ".evans-notes",
                "daily",
                `${date}.md`
              );
              try {
                const content = await readFile(notePath, "utf-8");
                result = `# Daily Note: ${date}\n\n${content}`;
              } catch (error) {
                result = `Daily note not found for ${date}. File: ${notePath}`;
              }
              break;
            }

            case "list_recent_claude_sessions": {
              const input = toolUse.input as { n?: number; project?: string };
              const n = input.n || 10;
              const projectFilter = input.project;

              try {
                const historyPath = join(homedir(), ".claude", "history.jsonl");
                const content = await readFile(historyPath, "utf-8");
                const lines = content.trim().split("\n");

                // Take last N lines, parse as JSON
                const sessions = lines
                  .slice(-n * 2) // Get more in case we filter
                  .map((line) => {
                    try {
                      return JSON.parse(line);
                    } catch {
                      return null;
                    }
                  })
                  .filter((s) => s !== null)
                  .filter((s) => {
                    if (!projectFilter) return true;
                    return s.project && s.project.includes(projectFilter);
                  })
                  .slice(-n); // Take last N after filtering

                if (sessions.length === 0) {
                  result = "No recent Claude Code sessions found.";
                } else {
                  result =
                    `# Recent Claude Code Sessions (${sessions.length})\n\n` +
                    sessions
                      .reverse()
                      .map((s, idx) => {
                        const timestamp = s.timestamp
                          ? new Date(s.timestamp).toLocaleString()
                          : "Unknown time";
                        return `${idx + 1}. **${timestamp}**\n   Project: ${s.project || "Unknown"}\n   ${s.display || "(No title)"}`;
                      })
                      .join("\n\n");
                }
              } catch (error) {
                result = `Error reading Claude history: ${error instanceof Error ? error.message : String(error)}`;
              }
              break;
            }

            case "search_dispatch": {
              const input = toolUse.input as {
                query: string;
                path?: string;
                limit?: number;
              };
              const { query, path, limit = 20 } = input;

              try {
                const searchPath = path
                  ? join(homedir(), "float-hub", "float.dispatch", path)
                  : join(homedir(), "float-hub", "float.dispatch");

                // Use grep for search
                const grepCmd = `grep -r -i -n "${query.replace(/"/g, '\\"')}" "${searchPath}" 2>/dev/null | head -${limit}`;
                const { stdout } = await execAsync(grepCmd);

                if (!stdout.trim()) {
                  result = `No matches found for "${query}" in ${path || "float.dispatch"}`;
                } else {
                  const lines = stdout.trim().split("\n");
                  result = `# Search Results: "${query}"\n\nFound ${lines.length} matches${path ? ` in ${path}` : ""}:\n\n${lines.join("\n")}`;
                }
              } catch (error) {
                result = `Error searching float.dispatch: ${error instanceof Error ? error.message : String(error)}`;
              }
              break;
            }

            case "read_file": {
              const input = toolUse.input as { path: string };
              let filePath = input.path;

              // Expand ~ to home directory
              if (filePath.startsWith("~/")) {
                filePath = join(homedir(), filePath.slice(2));
              }

              // Basic path validation (must be absolute)
              if (!filePath.startsWith("/")) {
                result = `Invalid path: ${input.path}. Path must be absolute (start with / or ~).`;
                break;
              }

              try {
                const content = await readFile(filePath, "utf-8");
                result = `# File: ${input.path}\n\n${content}`;
              } catch (error) {
                result = `Error reading file ${input.path}: ${error instanceof Error ? error.message : String(error)}`;
              }
              break;
            }

            default:
              result = `Unknown tool: ${toolUse.name}`;
          }
        } catch (error) {
          console.error(
            `[ask_evna] Error executing ${toolUse.name}:`,
            error
          );
          result = `Error executing ${toolUse.name}: ${error instanceof Error ? error.message : String(error)}`;
        }

        // Track this search attempt for early termination logic
        if (this.searchSession) {
          const resultsFound = !result.includes("No results found") &&
                               !result.includes("No matches found") &&
                               !result.includes("Error");

          // Use LLM to score quality semantically
          const quality = await this.scoreResultQuality(
            this.searchSession.getQuery(),
            toolUse.name,
            result
          );

          console.error(`[ask_evna] Quality score for ${toolUse.name}: ${quality}`);

          this.searchSession.addAttempt({
            tool: toolUse.name,
            input: toolUse.input,
            resultsFound,
            resultQuality: quality,
            tokenCost: 0, // Will be updated with actual token cost from usage
            timestamp: new Date().toISOString(),
          });
        }

        return {
          type: "tool_result" as const,
          tool_use_id: toolUse.id,
          content: result,
        };
      })
    );

    return results;
  }

  /**
   * Extract text response from Claude's message
   */
  private extractTextResponse(response: Anthropic.Message): string {
    const textBlocks = response.content.filter(
      (block): block is Anthropic.TextBlock => block.type === "text"
    );

    if (textBlocks.length === 0) {
      return "No response generated";
    }

    return textBlocks.map((block) => block.text).join("\n\n");
  }

  /**
   * Define tools available to the orchestrator agent
   * These mirror the existing tool schemas but in Anthropic format
   */
  private defineTools(): Anthropic.Tool[] {
    return [
      {
        name: "active_context",
        description:
          'Query recent activity stream (last few hours to days). Use for "what am I working on now?" or "recent work" queries. Supports project filtering.',
        input_schema: {
          type: "object",
          properties: {
            query: {
              type: "string",
              description: "Optional search query for filtering context",
            },
            project: {
              type: "string",
              description: "Filter by project name (e.g., 'pharmacy', 'floatctl')",
            },
            limit: {
              type: "number",
              description: "Maximum number of results (default: 10)",
            },
          },
        },
      },
      {
        name: "semantic_search",
        description:
          "Deep semantic search across conversation history. Use for finding past discussions, patterns across time, or specific topics. Searches entire archive.",
        input_schema: {
          type: "object",
          properties: {
            query: {
              type: "string",
              description: "Search query (natural language or keywords)",
            },
            limit: {
              type: "number",
              description: "Maximum number of results (default: 10)",
            },
            project: {
              type: "string",
              description: "Filter by project name",
            },
            threshold: {
              type: "number",
              description:
                "Similarity threshold 0-1 (default: 0.5, lower = more results)",
            },
          },
          required: ["query"],
        },
      },
      {
        name: "brain_boot",
        description:
          'Multi-source synthesis (semantic search + GitHub + daily notes + recent activity). Use for comprehensive context restoration: morning check-ins, "where did I leave off?", returning from breaks.',
        input_schema: {
          type: "object",
          properties: {
            query: {
              type: "string",
              description:
                "Natural language description of what to retrieve context about",
            },
            project: {
              type: "string",
              description: "Filter by project name (e.g., 'pharmacy')",
            },
            lookbackDays: {
              type: "number",
              description: "How many days to look back (default: 7)",
            },
            maxResults: {
              type: "number",
              description: "Maximum results to return (default: 10)",
            },
            githubUsername: {
              type: "string",
              description:
                "GitHub username to fetch PR and issue status (e.g., 'e-schultz')",
            },
          },
          required: ["query"],
        },
      },
      {
        name: "read_daily_note",
        description:
          "Read Evan's daily notes from ~/.evans-notes/daily/. Defaults to today if no date specified. Daily notes contain: timelog, tasks, reminders, invoice tracking, meeting notes.",
        input_schema: {
          type: "object",
          properties: {
            date: {
              type: "string",
              description:
                'Date in YYYY-MM-DD format. Omit for today. Examples: "2025-10-30", "2025-12-22"',
            },
          },
        },
      },
      {
        name: "list_recent_claude_sessions",
        description:
          "List recent Claude Code conversation sessions from ~/.claude/history.jsonl. Shows project paths and timestamps. Use for 'what conversations did I have?' or 'recent Claude sessions'.",
        input_schema: {
          type: "object",
          properties: {
            n: {
              type: "number",
              description: "Number of recent sessions to return (default: 10)",
            },
            project: {
              type: "string",
              description:
                'Optional project path filter (e.g., "floatctl-rs" will match "/Users/evan/float-hub-operations/floatctl-rs")',
            },
          },
        },
      },
      {
        name: "search_dispatch",
        description:
          "Search ~/float-hub/float.dispatch for content. Searches inbox, imprints (slutprints, sysops-daydream, the-field-guide, etc.). Use for finding specific topics, patterns, or files in Evan's knowledge base.",
        input_schema: {
          type: "object",
          properties: {
            query: {
              type: "string",
              description: "Search query (will use grep -i for case-insensitive)",
            },
            path: {
              type: "string",
              description:
                'Optional subdirectory to search (e.g., "inbox", "imprints/slutprints"). Omit to search entire dispatch.',
            },
            limit: {
              type: "number",
              description:
                "Maximum number of matching lines to return (default: 20)",
            },
          },
          required: ["query"],
        },
      },
      {
        name: "read_file",
        description:
          "Read any file by absolute path. Use when you need specific file content and have the exact path. Paths must be absolute (start with / or ~).",
        input_schema: {
          type: "object",
          properties: {
            path: {
              type: "string",
              description:
                'Absolute file path. Examples: "/Users/evan/float-hub/float.dispatch/inbox/2025-10-27-daddy-claude.md", "~/.evans-notes/daily/2025-10-30.md"',
            },
          },
          required: ["path"],
        },
      },
    ];
  }
}
