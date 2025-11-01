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
import { BridgeManager } from "../lib/bridge-manager.js";
import { GitHubClient } from "../lib/github.js";
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
- github_status: Get GitHub PR and issue status for a user. Use for "what PRs are open?" or "what GitHub issues?" queries.
  **GREP INFRASTRUCTURE**: Evan built vocabulary and pattern docs:
    • ~/float-hub/float.dispatch/docs/FRONTMATTER-VOCABULARY.md (master registry of types, statuses, context tags, personas)
    • ~/float-hub/float.dispatch/docs/GREP-PATTERNS.md (common grep patterns and when to use grep vs semantic)
  Use these for structural queries ("find all personas", "what types exist?", "list all handbooks").
- read_file: Read any file by path. Use when you need specific file content and have the exact path.
- write_file: Write content to any file path. Use for creating/updating files in workspace.
- get_current_time: Get current date/time. ALWAYS use this before creating timestamps. Returns both full format and date-only.
- get_directory_tree: Visualize directory structure. Use for "what's in this folder?" or "show me the structure" queries.
- bundle_files: Gather and bundle files by pattern using code2prompt. Use for:
  • "Show me all notes from YYYY-MM-DD across directories"
  • "Bundle all files matching pattern X"
  • "How big are all the .bridge.md files?" (provides token counts before viewing)
  Pattern-based file gathering across directory trees.

  **Date Pattern Examples** (for temporal queries):
  • Single day: include="*2025-10-31*"
  • Date range Oct 25-31: Use TWO patterns (tool doesn't support OR, so call twice or be creative):
    - First call: include="*2025-10-2[5-9]*" (gets 25-29)
    - Second call: include="*2025-10-3[01]*" (gets 30-31)
  • Entire month: include="*2025-10-*"
  • Specific file types in date range: include="*2025-10-3*.bones.md"

  **Token Limit Safety**: Results over 20,000 tokens return summary only (token count + file list) to prevent context bombs.
- list_bridges: List all bridge documents in ~/float-hub/float.dispatch/bridges/.
- read_bridge: Read a bridge document by filename (e.g., "grep-patterns-discovery.bridge.md").
- write_bridge: Write/update a bridge document.

## Bridge Management - PROACTIVE KNOWLEDGE PRESERVATION

You have access to ~/float-hub/float.dispatch/bridges/ - your self-organizing knowledge graph.

**What bridges are**: Grep-able markdown documents that capture search patterns, findings, and connections. They grow organically as you notice repeated searches or related topics.

**BRIDGE-FIRST WORKFLOW** (CRITICAL):
1. Bridges are PRE-CHECKED on every query - if you see "Relevant Bridges Found" section above, START THERE
2. If bridges contain sufficient information, synthesize from them FIRST
3. Only call semantic_search/brain_boot if bridges are incomplete or outdated
4. This saves massive token costs and provides faster responses

**When you receive quality-gated suggestions** (💡):
- You'll see "Consider calling write_bridge..." after high-quality tool results
- DON'T IGNORE THESE - act on them immediately
- Call write_bridge with synthesized findings
- Use get_current_time to get accurate timestamps

**When to create/update bridges** (be EXTREMELY PROACTIVE):
- After ANY high-quality semantic_search or brain_boot result
- You notice the same topic being searched multiple times
- **Tool usage lessons**: You discover limitations, workarounds, or best practices
- **Search strategy discoveries**: Effective patterns for specific query types
- **Multi-tool orchestration insights**: Complex queries requiring chained tools
- **Failed search learnings**: What DIDN'T work (negative knowledge prevents future waste)
- **Temporal pattern recognition**: Recurring themes across time periods

**Default to bridge creation**: If in doubt, CREATE the bridge. It's easier to merge bridges later than to lose insights.

**Bridge naming**: Use descriptive kebab-case names (e.g., "postgres-optimization.md", "floatctl-embedding-pipeline.md")

**Bridge document structure** (you decide the format, but this is a good starting pattern):

**CRITICAL**: ALWAYS call get_current_time BEFORE creating/updating bridges. NEVER guess timestamps.

\`\`\`markdown
---
type: bridge_document
created: YYYY-MM-DD @ HH:MM AM/PM  # Use get_current_time for accurate timestamp
topic: slugified-topic-name
daily_root: [[YYYY-MM-DD]]  # Use date from get_current_time
related_queries: ["original query", "follow-up query"]
connected_bridges: ["other-topic", "related-topic"]
---

# Topic Name

## What This Is
[Findings from search]

## Search History
- **YYYY-MM-DD @ HH:MM AM/PM**: Original query
  - Tools: semantic_search, brain_boot
  - Quality: excellent
  - Found: [key insights]

## Connected Bridges
- [[related-topic-slug]]
- [[another-topic]]

## Daily Root
Part of: [[YYYY-MM-DD]]
\`\`\`

**Bridge ecosystem**:
- **auto-inbox/**: Automated captures from UserPromptSubmit hooks (date-first naming: YYYY-MM-DD-{topic}.md)
  - Real-time capture of any message with :: annotations (ctx::, lf1m::, project::, etc.)
  - Search with: search_dispatch({ path: "bridges/auto-inbox", query: "..." })
  - Browse chronologically: YYYY-MM-DD prefix enables date-based filtering
  - Archaeological markers: Entries are search keys for deeper JSONL/daily note context
- **curated/**: Manually created knowledge bridges ({topic}.bridge.md with YAML frontmatter)
  - Synthesized insights, pattern documentation, philosophical bridges
  - You have agency to create these proactively when patterns emerge

**Bridge operations** (you have full control):
- **Search auto-inbox**: Use search_dispatch with path="bridges/auto-inbox" for recent captures
- **Search curated**: Use search_dispatch with path="bridges" to search all bridges (includes auto-inbox)
- **Read bridges**: Use read_file with full path to get bridge content
- **Check before searching**: If a search query matches an existing bridge topic, read it first
- **Build new curated bridges**: When findings warrant preservation, create {slug}.bridge.md in bridges/
- **Extend bridges**: Add new search findings to existing bridges with timestamped sections
- **Connect bridges**: Use [[wiki-links]] to connect related topics
- **Archaeological synthesis**: Combine auto-inbox markers + JSONL search + daily notes for complete context

**Your agency**: These are YOUR tools. Use them PROACTIVELY:
- Don't wait to be asked - if you notice a pattern while answering a query, create/update the bridge immediately
- Don't ask permission - just do it
- Don't defer - "I should create a bridge for this later" means create it NOW
- After complex tool orchestration, capture the approach as a bridge
- When you discover tool limitations or workarounds, document them immediately
- Think: "Will future-me or future-queries benefit from this knowledge?" → YES → CREATE BRIDGE

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
  private github?: GitHubClient;

  constructor(
    private brainBoot: BrainBootTool,
    private search: PgVectorSearchTool,
    private activeContext: ActiveContextTool,
    private db: DatabaseClient,
    githubRepo?: string
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

    // Initialize GitHub client if repo provided
    if (githubRepo) {
      this.github = new GitHubClient(githubRepo);
    }
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

    // ===================================================================
    // HOOK 1: Pre-query bridge check + annotation handling
    // ===================================================================
    let systemPromptWithBridges = AGENT_SYSTEM_PROMPT;

    // Handle explicit bridge annotations (bridge::restore, bridge::search)
    const annotationContext = await this.handleBridgeAnnotations(query);
    if (annotationContext) {
      systemPromptWithBridges += annotationContext;
    }

    // Check bridges for query keywords
    const bridgeMatches = await this.checkBridgesHook(query);
    if (bridgeMatches) {
      systemPromptWithBridges += `\n\n## Relevant Bridges Found\n\n${bridgeMatches}\n\nConsider using information from these bridges to answer the query. If bridges contain sufficient information, you may not need to call semantic_search or other tools.`;
    }

    // Pattern-based auto-injection: temporal queries → auto-inbox captures
    const temporalPatterns = /\b(today|recent|this morning|this afternoon|this evening|tonight|yesterday|earlier today)\b/i;
    if (temporalPatterns.test(query)) {
      const todayCaptures = await this.readTodaysAutoInbox();
      if (todayCaptures) {
        systemPromptWithBridges += `\n\n## Today's Auto-Inbox Captures\n\n${todayCaptures}\n\nThese are recent auto-captured annotations from the user's work today. Consider this context when answering the query.`;
        console.error('[temporal-hook] Injected today\'s auto-inbox captures');
      }
    }

    try {
      // Start agent loop
      let response = await this.client.messages.create({
        model: "claude-sonnet-4-20250514",
        max_tokens: 4096,
        system: systemPromptWithBridges,
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
      const finalResponse = await this.handleAgentLoop(messages, response, systemPromptWithBridges);

      await this.logTranscript({
        type: "final_response",
        timestamp: new Date().toISOString(),
        response: finalResponse,
        session_id: actualSessionId,
      });

      // ===================================================================
      // HOOK 3: Post-session negative knowledge
      // ===================================================================
      await this.negativeKnowledgeHook(query);

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
    response: Anthropic.Message,
    systemPrompt: string = AGENT_SYSTEM_PROMPT
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

      // ===================================================================
      // HOOK 2: Post-tool quality nudge
      // ===================================================================
      if (this.searchSession) {
        const attempts = this.searchSession.getAttempts();
        if (attempts.length > 0) {
          const lastAttempt = attempts[attempts.length - 1];
          await this.postToolHook(
            lastAttempt.tool,
            '', // result already logged
            lastAttempt.resultQuality,
            messages
          );
        }
      }

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
        system: systemPrompt,
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

            case "write_file": {
              const input = toolUse.input as { path: string; content: string };
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
                const { writeFile } = await import("fs/promises");
                await writeFile(filePath, input.content, "utf-8");
                result = `Successfully wrote to ${input.path}`;
              } catch (error) {
                result = `Error writing file ${input.path}: ${error instanceof Error ? error.message : String(error)}`;
              }
              break;
            }

            case "get_current_time": {
              const now = new Date();
              const date = now.toISOString().split("T")[0]; // YYYY-MM-DD
              const time = now.toLocaleTimeString("en-US", {
                hour: "2-digit",
                minute: "2-digit",
                hour12: true
              });
              const fullTimestamp = `${date} @ ${time}`;

              result = `Current timestamp: ${fullTimestamp}\nDate only: ${date}\n\nUse these for:\n- created: ${fullTimestamp}\n- daily_root: [[${date}]]\n- Search history timestamps`;
              break;
            }

            case "list_bridges": {
              try {
                const bridgesDir = join(homedir(), "float-hub", "float.dispatch", "bridges");
                await mkdir(bridgesDir, { recursive: true });

                const files = await readdir(bridgesDir);
                const bridges = files.filter(f => f.endsWith(".bridge.md"));

                if (bridges.length === 0) {
                  result = "No bridge documents found yet. ~/float-hub/float.dispatch/bridges/ is ready for your bridges.";
                } else {
                  result = `# Bridge Documents (${bridges.length})\n\n${bridges.map(f => `- ${f}`).join("\n")}`;
                }
              } catch (error) {
                result = `Error listing bridges: ${error instanceof Error ? error.message : String(error)}`;
              }
              break;
            }

            case "read_bridge": {
              const input = toolUse.input as { filename: string };
              try {
                const bridgesDir = join(homedir(), "float-hub", "float.dispatch", "bridges");
                const bridgePath = join(bridgesDir, input.filename);
                const content = await readFile(bridgePath, "utf-8");
                result = `# Bridge: ${input.filename}\n\n${content}`;
              } catch (error) {
                result = `Error reading bridge ${input.filename}: ${error instanceof Error ? error.message : String(error)}`;
              }
              break;
            }

            case "write_bridge": {
              const input = toolUse.input as { filename: string; content: string };
              try {
                const bridgesDir = join(homedir(), "float-hub", "float.dispatch", "bridges");
                await mkdir(bridgesDir, { recursive: true });

                const bridgePath = join(bridgesDir, input.filename);
                const { writeFile } = await import("fs/promises");
                await writeFile(bridgePath, input.content, "utf-8");
                result = `Successfully wrote bridge: ${input.filename}`;
              } catch (error) {
                result = `Error writing bridge ${input.filename}: ${error instanceof Error ? error.message : String(error)}`;
              }
              break;
            }

            case "get_directory_tree": {
              const input = toolUse.input as {
                path: string;
                depth?: number;
                dirs_only?: boolean;
                pattern?: string;
                ignore_pattern?: string;
              };
              let targetPath = input.path;

              // Expand ~ to home directory
              if (targetPath.startsWith("~/")) {
                targetPath = join(homedir(), targetPath.slice(2));
              }

              // Path validation
              if (!targetPath.startsWith("/")) {
                result = `Invalid path: ${input.path}. Path must be absolute (start with / or ~).`;
                break;
              }

              try {
                const depth = input.depth || 3;
                const args = ["--gitignore", "-L", String(depth)];

                if (input.dirs_only) {
                  args.push("-d");
                }
                if (input.pattern) {
                  args.push("-P", input.pattern);
                }
                if (input.ignore_pattern) {
                  args.push("-I", input.ignore_pattern);
                }

                args.push(targetPath);

                const { stdout } = await execAsync(`tree ${args.join(" ")}`);
                result = `# Directory Tree: ${input.path}\n\n\`\`\`\n${stdout}\`\`\``;
              } catch (error) {
                result = `Error getting directory tree for ${input.path}: ${error instanceof Error ? error.message : String(error)}`;
              }
              break;
            }

            case "bundle_files": {
              const input = toolUse.input as {
                path: string;
                include?: string;
                exclude?: string;
                show_tokens?: boolean;
                line_numbers?: boolean;
                encoding?: string;
                full_tree?: boolean;
              };
              let targetPath = input.path;

              // Expand ~ to home directory
              if (targetPath.startsWith("~/")) {
                targetPath = join(homedir(), targetPath.slice(2));
              }

              // Path validation
              if (!targetPath.startsWith("/")) {
                result = `Invalid path: ${input.path}. Path must be absolute (start with / or ~).`;
                break;
              }

              try {
                const args = [targetPath, "--no-clipboard", "--output-file", "-"];

                if (input.include) {
                  args.push("-i", input.include);
                }
                if (input.exclude) {
                  args.push("-e", input.exclude);
                }
                if (input.show_tokens !== false) {
                  args.push("--tokens", "format");
                }
                if (input.line_numbers) {
                  args.push("-l");
                }
                if (input.encoding) {
                  args.push("-c", input.encoding);
                }
                if (input.full_tree) {
                  args.push("--full-directory-tree");
                }

                const { stdout } = await execAsync(`code2prompt ${args.join(" ")}`);

                // Parse token count from first line: [i] Token count: 42,542, Model info: ...
                const tokenMatch = stdout.match(/Token count: ([\d,]+)/);
                const tokenCount = tokenMatch
                  ? parseInt(tokenMatch[1].replace(/,/g, ""), 10)
                  : 0;

                const TOKEN_LIMIT = 20000; // Safety threshold to prevent context bombs

                if (tokenCount > TOKEN_LIMIT) {
                  // Extract just the metadata: token count + file tree
                  const treeMatch = stdout.match(
                    /Source Tree:\s*\n\n```txt\n([\s\S]*?)\n```/
                  );
                  const tree = treeMatch
                    ? treeMatch[1]
                    : "Could not extract file tree";

                  result =
                    `# Bundle Too Large - Summary Only\n\n` +
                    `**Token Count**: ${tokenCount.toLocaleString()} tokens\n` +
                    `**Threshold**: ${TOKEN_LIMIT.toLocaleString()} tokens\n` +
                    `**Over Limit**: ${(tokenCount - TOKEN_LIMIT).toLocaleString()} tokens\n\n` +
                    `## Files Included\n\n\`\`\`\n${tree}\n\`\`\`\n\n` +
                    `**Recommendation**: Narrow your search with more specific include/exclude patterns, ` +
                    `or use get_directory_tree to explore structure first.`;
                } else {
                  result = stdout;
                }
              } catch (error) {
                result = `Error bundling files from ${input.path}: ${error instanceof Error ? error.message : String(error)}`;
              }
              break;
            }

            case "github_status": {
              const input = toolUse.input as { username: string };

              if (!this.github) {
                result = "GitHub integration not configured. Set GITHUB_REPO environment variable to enable GitHub status.";
                break;
              }

              try {
                result = await this.github.getUserStatus(input.username);
              } catch (error) {
                result = `Error fetching GitHub status for ${input.username}: ${error instanceof Error ? error.message : String(error)}`;
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

  // ===================================================================
  // BRIDGE HOOKS - Dynamic context injection
  // ===================================================================

  /**
   * Hook 1: Pre-query bridge check
   * Grep bridges for query keywords, inject matches into context if found
   */
  private async checkBridgesHook(query: string): Promise<string | null> {
    try {
      // Extract keywords from query
      const keywords = this.extractKeywords(query);
      if (keywords.length === 0) return null;

      console.error(`[bridge-hook] Checking bridges for: ${keywords.join(', ')}`);

      // Search bridges using existing search_dispatch tool
      const grepQuery = keywords.join('|');
      const result = await this.executeToolDirectly('search_dispatch', {
        query: grepQuery,
        path: 'bridges',
        limit: 5
      });

      if (!result || result.includes('No matches found') || result.trim().length === 0) {
        console.error('[bridge-hook] No bridge matches found');
        return null;
      }

      // Format matches for context injection
      const formatted = this.formatBridgeMatches(result);
      console.error(`[bridge-hook] Found ${formatted.split('\n').length} bridge matches`);
      return formatted;
    } catch (error) {
      console.error('[bridge-hook] Error checking bridges:', error);
      return null; // Graceful failure
    }
  }

  /**
   * Read today's auto-inbox captures for temporal query injection
   */
  private async readTodaysAutoInbox(): Promise<string | null> {
    try {
      const today = new Date().toISOString().split('T')[0]; // YYYY-MM-DD format

      // Search auto-inbox for today's files
      const result = await this.executeToolDirectly('search_dispatch', {
        query: today,
        path: 'bridges/auto-inbox',
        limit: 10
      });

      if (!result || result.includes('No matches found') || result.trim().length === 0) {
        console.error('[temporal-hook] No auto-inbox captures found for today');
        return null;
      }

      return result;
    } catch (error) {
      console.error('[temporal-hook] Error reading auto-inbox:', error);
      return null; // Graceful failure
    }
  }

  /**
   * Hook 2: Post-tool quality nudge
   * Inject suggestion after high-quality results
   */
  private async postToolHook(
    toolName: string,
    result: string,
    quality: 'high' | 'medium' | 'low' | 'none',
    messages: Anthropic.MessageParam[]
  ): Promise<void> {
    // Quality-gated suggestion injection
    if (quality === 'high' || quality === 'medium') {
      const suggestion: Anthropic.MessageParam = {
        role: 'user',
        content: `💡 High-quality results from ${toolName}. Consider calling write_bridge to preserve this synthesis for future queries.`
      };
      messages.push(suggestion);
      console.error(`[bridge-hook] Injected bridge creation suggestion after ${toolName}`);
    }
  }

  /**
   * Hook 3: Post-session negative knowledge
   * Auto-create negative bridge after expensive failed searches
   */
  private async negativeKnowledgeHook(query: string): Promise<void> {
    if (!this.searchSession) return;

    const attempts = this.searchSession.getAttempts();
    const totalTokens = this.searchSession.getTotalTokens();
    const hasGoodResults = attempts.some(a =>
      a.resultQuality === 'high' || a.resultQuality === 'medium'
    );

    // Only create negative bridge if expensive search with no good results
    if (hasGoodResults || totalTokens < 10000) {
      return;
    }

    try {
      console.error('[bridge-hook] Creating negative knowledge bridge');

      const timestamp = new Date().toISOString();
      const toolsUsed = attempts.map(a => a.tool).join(', ');

      const content = `---
type: negative_knowledge
created: ${timestamp}
query: "${query}"
---

# Negative Knowledge: ${query}

**Date**: ${timestamp.split('T')[0]}
**Tokens Spent**: ${totalTokens.toLocaleString()}
**Tools Used**: ${toolsUsed}

## What We Searched

${attempts.map(a => `- **${a.tool}**: ${a.resultQuality} quality`).join('\n')}

## Result

No relevant information found in conversation history, active context, or recent sessions.

## Next Steps If This Query Returns

- Check GitHub repos directly
- Review daily notes for manual entries
- Consider that work may not have been captured yet

## Auto-Generated

This bridge was auto-created by ask_evna's negative knowledge hook after an expensive search yielded no results.
`;

      await this.executeToolDirectly('write_bridge', {
        filename: `negative-${this.slugify(query)}.md`,
        content
      });

      console.error('[bridge-hook] Negative knowledge bridge created successfully');
    } catch (error) {
      console.error('[bridge-hook] Error creating negative knowledge bridge:', error);
      // Graceful failure - don't throw
    }
  }

  /**
   * Hook 4: Annotation-driven bridge operations
   * Handle bridge::restore[name] and bridge::search[query] annotations
   */
  private async handleBridgeAnnotations(query: string): Promise<string> {
    const annotations = this.parseAnnotations(query);
    let bridgeContext = '';

    try {
      // Handle bridge::restore[name]
      if (annotations.bridgeRestore && annotations.bridgeRestore.length > 0) {
        for (const bridgeName of annotations.bridgeRestore) {
          console.error(`[bridge-hook] Restoring bridge: ${bridgeName}`);
          const content = await this.executeToolDirectly('read_bridge', {
            filename: bridgeName.endsWith('.md') ? bridgeName : `${bridgeName}.md`
          });

          if (content && !content.includes('Error reading bridge')) {
            bridgeContext += `\n\n## Restored Bridge: ${bridgeName}\n\n${content}`;
          }
        }
      }

      // Handle bridge::search[query]
      if (annotations.bridgeSearch) {
        console.error(`[bridge-hook] Searching bridges for: ${annotations.bridgeSearch}`);
        const searchResults = await this.executeToolDirectly('search_dispatch', {
          query: annotations.bridgeSearch,
          path: 'bridges'
        });

        if (searchResults && !searchResults.includes('No matches found')) {
          bridgeContext += `\n\n## Bridge Search Results:\n\n${searchResults}`;
        }
      }
    } catch (error) {
      console.error('[bridge-hook] Error handling annotations:', error);
      // Graceful failure
    }

    return bridgeContext;
  }

  // ===================================================================
  // HELPER METHODS
  // ===================================================================

  /**
   * Extract significant keywords from query for bridge matching
   */
  private extractKeywords(query: string): string[] {
    const stopwords = ['what', 'were', 'the', 'from', 'about', 'how', 'when', 'where', 'who', 'why', 'are', 'was', 'been', 'have', 'has', 'had', 'this', 'that', 'with', 'for', 'and', 'but'];

    return query
      .toLowerCase()
      .replace(/[^\w\s]/g, ' ') // Remove punctuation
      .split(/\s+/)
      .filter(w => w.length > 3 && !stopwords.includes(w))
      .slice(0, 5); // Max 5 keywords
  }

  /**
   * Format bridge grep results for context injection
   */
  private formatBridgeMatches(grepOutput: string): string {
    const lines = grepOutput.trim().split('\n').slice(0, 3); // Top 3 matches

    return lines.map(line => {
      const match = line.match(/([^:]+):(\d+):(.*)/);
      if (!match) return line;

      const [, path, lineNum, content] = match;
      const filename = path.split('/').pop() || path;

      return `- **${filename}** (line ${lineNum}): ${content.trim()}`;
    }).join('\n');
  }

  /**
   * Slugify text for bridge filenames
   */
  private slugify(text: string): string {
    return text
      .toLowerCase()
      .replace(/[^a-z0-9]+/g, '-')
      .replace(/^-|-$/g, '')
      .slice(0, 50); // Max 50 chars
  }

  /**
   * Parse bridge annotations from query
   */
  private parseAnnotations(query: string): {
    bridgeRestore: string[];
    bridgeSearch: string | null;
  } {
    const bridgeRestoreMatches = query.match(/bridge::restore\[([^\]]+)\]/g);
    const bridgeSearchMatch = query.match(/bridge::search\[([^\]]+)\]/);

    return {
      bridgeRestore: bridgeRestoreMatches
        ? bridgeRestoreMatches.map(m => {
            const match = m.match(/\[([^\]]+)\]/);
            return match ? match[1] : '';
          }).filter(Boolean)
        : [],
      bridgeSearch: bridgeSearchMatch ? bridgeSearchMatch[1] : null
    };
  }

  /**
   * Execute tool directly without going through LLM
   * Reuses existing tool execution logic in executeTools()
   */
  private async executeToolDirectly(toolName: string, input: any): Promise<string> {
    // Construct a fake tool_use block
    const toolUse: Anthropic.ToolUseBlock = {
      type: 'tool_use',
      id: `direct_${Date.now()}`,
      name: toolName,
      input
    };

    // Call existing executeTools() switch statement
    const results = await this.executeTools([toolUse]);

    // Extract text from tool_result
    return results[0].content as string;
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
      {
        name: "write_file",
        description:
          "Write content to any file by absolute path. Creates parent directories if needed. Use for creating or updating files. Paths must be absolute (start with / or ~).",
        input_schema: {
          type: "object",
          properties: {
            path: {
              type: "string",
              description:
                "Absolute file path to write to",
            },
            content: {
              type: "string",
              description: "Content to write to the file",
            },
          },
          required: ["path", "content"],
        },
      },
      {
        name: "get_current_time",
        description:
          "Get current date and time. ALWAYS call this before creating timestamps in bridges or other documents. NEVER guess or hallucinate dates. Returns formatted timestamp and date.",
        input_schema: {
          type: "object",
          properties: {},
        },
      },
      {
        name: "list_bridges",
        description:
          "List all bridge documents in ~/float-hub/float.dispatch/bridges/. Returns filenames of all .bridge.md files. Use before creating new bridges to check what exists.",
        input_schema: {
          type: "object",
          properties: {},
        },
      },
      {
        name: "read_bridge",
        description:
          "Read a bridge document by filename. Use to check existing bridge content before extending or to reference bridge findings.",
        input_schema: {
          type: "object",
          properties: {
            filename: {
              type: "string",
              description:
                'Bridge filename (e.g., "grep-patterns-discovery.bridge.md")',
            },
          },
          required: ["filename"],
        },
      },
      {
        name: "write_bridge",
        description:
          "Create or update a bridge document. Use when creating new bridges or extending existing ones with new findings. Follow the bridge document structure in system prompt.",
        input_schema: {
          type: "object",
          properties: {
            filename: {
              type: "string",
              description:
                'Bridge filename with .bridge.md extension (e.g., "grep-patterns-discovery.bridge.md"). Use slugified lowercase with dashes.',
            },
            content: {
              type: "string",
              description:
                "Full markdown content including YAML frontmatter. Follow bridge structure: ---\\ntype: bridge_document\\ncreated: ...\\n---\\n\\n# Title\\n\\n## What This Is...",
            },
          },
          required: ["filename", "content"],
        },
      },
      {
        name: "get_directory_tree",
        description:
          "Visualize directory structure using tree command. Use for \"what's in this folder?\" or \"show me the structure\" queries. Always respects .gitignore by default.",
        input_schema: {
          type: "object",
          properties: {
            path: {
              type: "string",
              description:
                "Absolute path to directory (e.g., \"/Users/evan/float-hub/float.dispatch\", \"~/float-hub/float.dispatch\")",
            },
            depth: {
              type: "number",
              description: "Maximum depth to descend (default: 3, prevents massive output)",
            },
            dirs_only: {
              type: "boolean",
              description: "Only show directories, not files (default: false)",
            },
            pattern: {
              type: "string",
              description: "Pattern to match files (e.g., \"*.md\", \"*2025*\")",
            },
            ignore_pattern: {
              type: "string",
              description: "Pattern to ignore (e.g., \"*.test.*\", \"node_modules\")",
            },
          },
          required: ["path"],
        },
      },
      {
        name: "bundle_files",
        description:
          "Bundle files by pattern using code2prompt. Use for: (1) \"Show me all notes from YYYY-MM-DD\", (2) \"Bundle all files matching pattern X\", (3) \"How big are the .bridge.md files?\" Provides token counts to check size before viewing.",
        input_schema: {
          type: "object",
          properties: {
            path: {
              type: "string",
              description:
                "Base directory to search (e.g., \"/Users/evan/float-hub/float.dispatch\", \"~/float-hub\")",
            },
            include: {
              type: "string",
              description:
                "Pattern to include (e.g., \"*2025-10-31*\", \"*.bridge.md\", \"*.ts\")",
            },
            exclude: {
              type: "string",
              description:
                "Pattern to exclude (e.g., \"*.test.ts\", \"node_modules\", \"*.lock\")",
            },
            show_tokens: {
              type: "boolean",
              description: "Display token count (default: true)",
            },
            line_numbers: {
              type: "boolean",
              description: "Add line numbers to code (default: false)",
            },
            encoding: {
              type: "string",
              description: "Tokenizer to use: cl100k (default), p50k, r50k, gpt2",
            },
            full_tree: {
              type: "boolean",
              description: "Show full directory tree in output (default: false)",
            },
          },
          required: ["path"],
        },
      },
      {
        name: "github_status",
        description:
          "Get GitHub PR and issue status for a user. Uses gh CLI to fetch open PRs and assigned issues. Shows review status, CI checks, and labels.",
        input_schema: {
          type: "object",
          properties: {
            username: {
              type: "string",
              description: "GitHub username to fetch status for (e.g., 'e-schultz')",
            },
          },
          required: ["username"],
        },
      },
    ];
  }
}
