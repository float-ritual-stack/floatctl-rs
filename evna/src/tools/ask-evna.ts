/**
 * Ask EVNA Tool
 * LLM-driven orchestration layer that interprets natural language queries
 * and intelligently coordinates existing evna tools
 */

import Anthropic from "@anthropic-ai/sdk";
import { BrainBootTool } from "./brain-boot.js";
import { PgVectorSearchTool } from "./pgvector-search.js";
import { ActiveContextTool } from "./active-context.js";

// System prompt for the orchestrator agent
const AGENT_SYSTEM_PROMPT = `You are evna, an agent orchestrator for Evan's work context system.

Available tools:
- active_context: Recent activity stream (last few hours to days). Use for "what am I working on now?" or "recent work" queries. Can filter by project.
- semantic_search: Deep historical search (full conversation archive). Use for finding past discussions, patterns across time, or specific topics regardless of when they occurred.
- brain_boot: Multi-source synthesis (semantic + GitHub + daily notes + recent activity). Use for comprehensive context restoration like morning check-ins, returning from breaks, or "where did I leave off?" scenarios.

Your job:
1. Understand the query intent (temporal? project-based? semantic? comprehensive?)
2. Decide which tool(s) to call (one or multiple)
3. Execute tools in appropriate order if chaining is needed
4. Synthesize results into coherent narrative
5. Filter noise (ignore irrelevant tangents)
6. Avoid repeating what user just said

Guidelines:
- For recent/temporal queries: Use active_context or brain_boot
- For historical/semantic queries: Use semantic_search
- For comprehensive "where did I leave off?" queries: Use brain_boot
- You can call multiple tools if needed (e.g., semantic_search then active_context for temporal refinement)

Respond with synthesis, not raw data dumps. Focus on answering the user's question directly.`;

export interface AskEvnaOptions {
  query: string;
}

export class AskEvnaTool {
  private client: Anthropic;

  constructor(
    private brainBoot: BrainBootTool,
    private search: PgVectorSearchTool,
    private activeContext: ActiveContextTool
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
   * Ask evna a natural language question
   * The orchestrator agent decides which tools to use
   */
  async ask(options: AskEvnaOptions): Promise<string> {
    const { query } = options;

    console.log("[ask_evna] Query:", query);

    try {
      // Create initial message to the orchestrator
      const messages: Anthropic.MessageParam[] = [
        {
          role: "user",
          content: query,
        },
      ];

      // Start agent loop
      let response = await this.client.messages.create({
        model: "claude-sonnet-4-20250514",
        max_tokens: 4096,
        system: AGENT_SYSTEM_PROMPT,
        messages,
        tools: this.defineTools(),
      });

      // Handle multi-turn tool execution
      const finalResponse = await this.handleAgentLoop(messages, response);

      return finalResponse;
    } catch (error) {
      console.error("[ask_evna] Error:", error);
      throw error;
    }
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
      console.log("[ask_evna] Agent requesting tool use");

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

      // Continue conversation with tool results
      currentResponse = await this.client.messages.create({
        model: "claude-sonnet-4-20250514",
        max_tokens: 4096,
        system: AGENT_SYSTEM_PROMPT,
        messages,
        tools: this.defineTools(),
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
        console.log(`[ask_evna] Executing tool: ${toolUse.name}`);

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
    ];
  }
}
